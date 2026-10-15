use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result, anyhow, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use rustls_platform_verifier::BuilderVerifierExt;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::{JoinHandle, JoinSet},
};
use tokio_rustls::{TlsConnector, rustls};
use url::Url;

// Webview 只连接无认证的本地 HTTP 代理；上游协议、认证和 TLS 在这里处理。
pub(super) struct VerificationProxy {
    pub url: Url,
    worker: JoinHandle<()>,
}

impl Drop for VerificationProxy {
    fn drop(&mut self) {
        // 结束监听任务时，JoinSet 同时取消此窗口的所有隧道。
        self.worker.abort();
    }
}

impl VerificationProxy {
    pub async fn start(proxy: &str) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            // WKWebView 的按窗口代理配置从 macOS 14 开始提供。
            let version = std::process::Command::new("/usr/bin/sw_vers")
                .arg("-productVersion")
                .output()
                .context("无法检查 macOS 验证代理支持情况")?;
            ensure!(
                version.status.success()
                    && String::from_utf8_lossy(&version.stdout)
                        .split('.')
                        .next()
                        .and_then(|major| major.parse::<u32>().ok())
                        .is_some_and(|major| major >= 14),
                "使用代理进行安全验证需要 macOS 14 或更高版本"
            );
        }
        let upstream = Url::parse(proxy).map_err(|_| anyhow!("验证代理地址无效"))?;
        ensure!(
            matches!(upstream.scheme(), "http" | "https" | "socks5"),
            "验证代理仅支持 HTTP、HTTPS 和 SOCKS5"
        );
        ensure!(upstream.host_str().is_some(), "验证代理缺少主机地址");
        let tls = if upstream.scheme() == "https" {
            Some(TlsConnector::from(Arc::new(
                rustls::ClientConfig::builder_with_provider(Arc::new(
                    rustls::crypto::aws_lc_rs::default_provider(),
                ))
                .with_safe_default_protocol_versions()?
                .with_platform_verifier()?
                .with_no_client_auth(),
            )))
        } else {
            None
        };
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
        let url = Url::parse(&format!("http://{}", listener.local_addr()?))?;
        let worker = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let Ok((stream, _)) = accepted else { break };
                        let upstream = upstream.clone();
                        let tls = tls.clone();
                        connections.spawn(async move {
                            if let Err(error) = relay(stream, &upstream, tls).await {
                                // 错误只记录固定上下文，不输出含密码的代理 URL。
                                tracing::warn!(error = %error, "验证代理隧道失败");
                            }
                        });
                    }
                    _ = connections.join_next(), if !connections.is_empty() => {}
                }
            }
        });
        Ok(Self { url, worker })
    }
}

trait ProxyStream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> ProxyStream for T {}

async fn read_headers(stream: &mut (impl AsyncRead + Unpin)) -> Result<String> {
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        ensure!(header.len() < 16384, "验证代理响应头过长");
        header.push(stream.read_u8().await?);
    }
    Ok(String::from_utf8(header)?)
}

async fn relay(mut browser: TcpStream, upstream: &Url, tls: Option<TlsConnector>) -> Result<()> {
    let connect = async {
        let request = read_headers(&mut browser).await?;
        let mut request_line = request
            .lines()
            .next()
            .unwrap_or_default()
            .split_whitespace();
        ensure!(
            request_line.next() == Some("CONNECT"),
            "验证代理只接受 HTTPS 隧道"
        );
        let target = request_line.next().ok_or_else(|| anyhow!("缺少隧道目标"))?;
        let target_url = Url::parse(&format!("https://{target}"))?;
        ensure!(
            target_url.host_str().is_some()
                && target_url.username().is_empty()
                && target_url.password().is_none()
                && target_url.path() == "/"
                && target_url.query().is_none()
                && target_url.fragment().is_none(),
            "验证代理隧道目标无效"
        );
        let host = upstream.host().unwrap().to_string();
        let port = upstream.port_or_known_default().unwrap_or(1080);
        let socket = TcpStream::connect((host.as_str(), port))
            .await
            .context("连接上游代理失败")?;
        let mut socket: Box<dyn ProxyStream> = if let Some(tls) = tls {
            Box::new(
                tls.connect(
                    rustls::pki_types::ServerName::try_from(host.to_owned())?,
                    socket,
                )
                .await
                .context("上游 HTTPS 代理 TLS 握手失败")?,
            )
        } else {
            Box::new(socket)
        };
        if upstream.scheme() == "socks5" {
            socks_connect(&mut socket, upstream, &target_url).await?;
        } else {
            let mut connect = format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n");
            if !upstream.username().is_empty() || upstream.password().is_some() {
                let (username, password) = credentials(upstream)?;
                let auth = STANDARD.encode(format!("{username}:{password}"));
                connect.push_str(&format!("Proxy-Authorization: Basic {auth}\r\n"));
            }
            connect.push_str("\r\n");
            socket.write_all(connect.as_bytes()).await?;
            let response = read_headers(&mut socket).await?;
            ensure!(
                response
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    == Some("200"),
                "上游代理拒绝 CONNECT，请检查代理认证和连接配置"
            );
        }
        Ok::<_, anyhow::Error>(socket)
    };
    let connected = if cfg!(debug_assertions) {
        Ok(connect.await)
    } else {
        tokio::time::timeout(Duration::from_secs(30), connect).await
    };
    let mut socket = match connected {
        Ok(Ok(socket)) => socket,
        result => {
            let _ = browser
                .write_all(b"HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\n\r\n")
                .await;
            return match result {
                Ok(Err(error)) => Err(error),
                _ => Err(anyhow!("连接验证代理超时")),
            };
        }
    };
    browser
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await?;
    tokio::io::copy_bidirectional(&mut browser, &mut socket).await?;
    Ok(())
}

fn credentials(proxy: &Url) -> Result<(String, String)> {
    // URL 用户信息中的 '+' 是字面值，不能按表单规则转换为空格。
    fn decode(value: &str) -> Result<String> {
        let encoded = format!("v={}", value.replace('+', "%2B").replace('&', "%26"));
        Ok(url::form_urlencoded::parse(encoded.as_bytes())
            .next()
            .ok_or_else(|| anyhow!("代理认证信息无效"))?
            .1
            .into_owned())
    }
    Ok((
        decode(proxy.username())?,
        decode(proxy.password().unwrap_or_default())?,
    ))
}

async fn socks_connect(socket: &mut Box<dyn ProxyStream>, proxy: &Url, target: &Url) -> Result<()> {
    let authenticated = !proxy.username().is_empty() || proxy.password().is_some();
    let method = if authenticated { 2 } else { 0 };
    socket.write_all(&[5, 1, method]).await?;
    let mut reply = [0; 2];
    socket.read_exact(&mut reply).await?;
    ensure!(reply == [5, method], "SOCKS5 代理不支持配置的认证方式");
    if authenticated {
        let (username, password) = credentials(proxy)?;
        ensure!(
            username.len() <= 255 && password.len() <= 255,
            "SOCKS5 认证信息过长"
        );
        let mut auth = vec![1, username.len() as u8];
        auth.extend_from_slice(username.as_bytes());
        auth.push(password.len() as u8);
        auth.extend_from_slice(password.as_bytes());
        socket.write_all(&auth).await?;
        socket.read_exact(&mut reply).await?;
        ensure!(reply == [1, 0], "SOCKS5 代理认证失败");
    }
    let mut connect = vec![5, 1, 0];
    match target.host().unwrap() {
        url::Host::Domain(host) => {
            ensure!(host.len() <= 255, "SOCKS5 目标域名过长");
            connect.extend_from_slice(&[3, host.len() as u8]);
            connect.extend_from_slice(host.as_bytes());
        }
        url::Host::Ipv4(ip) => {
            connect.push(1);
            connect.extend_from_slice(&ip.octets());
        }
        url::Host::Ipv6(ip) => {
            connect.push(4);
            connect.extend_from_slice(&ip.octets());
        }
    }
    connect.extend_from_slice(&target.port_or_known_default().unwrap_or(443).to_be_bytes());
    socket.write_all(&connect).await?;
    let mut response = [0; 4];
    socket.read_exact(&mut response).await?;
    ensure!(response[0..3] == [5, 0, 0], "SOCKS5 代理连接目标失败");
    let length = match response[3] {
        1 => 4,
        3 => socket.read_u8().await? as usize,
        4 => 16,
        _ => return Err(anyhow!("SOCKS5 代理响应地址无效")),
    };
    socket.read_exact(&mut vec![0; length + 2]).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn browser(proxy: &VerificationProxy) -> TcpStream {
        let mut browser = TcpStream::connect(("127.0.0.1", proxy.url.port().unwrap()))
            .await
            .unwrap();
        browser
            .write_all(b"CONNECT member.bilibili.com:443 HTTP/1.1\r\nHost: member.bilibili.com:443\r\n\r\n")
            .await
            .unwrap();
        browser
    }

    #[tokio::test]
    async fn http_proxy_preserves_credentials_and_relays_both_directions() {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = upstream.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = upstream.accept().await.unwrap();
            let headers = read_headers(&mut stream).await.unwrap();
            assert!(headers.starts_with("CONNECT member.bilibili.com:443 HTTP/1.1\r\n"));
            let auth = STANDARD.encode("user+&:密:码");
            assert!(headers.contains(&format!("Proxy-Authorization: Basic {auth}\r\n")));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\n\r\nserver")
                .await
                .unwrap();
            let mut received = [0; 6];
            stream.read_exact(&mut received).await.unwrap();
            assert_eq!(&received, b"client");
        });
        let proxy =
            VerificationProxy::start(&format!("http://user%2B&:%E5%AF%86%3A%E7%A0%81@{address}"))
                .await
                .unwrap();
        let mut browser = browser(&proxy).await;
        assert!(
            read_headers(&mut browser)
                .await
                .unwrap()
                .starts_with("HTTP/1.1 200")
        );
        let mut received = [0; 6];
        browser.read_exact(&mut received).await.unwrap();
        assert_eq!(&received, b"server");
        browser.write_all(b"client").await.unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn socks_proxy_authenticates_and_preserves_remote_hostname() {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = upstream.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = upstream.accept().await.unwrap();
            let mut greeting = [0; 3];
            stream.read_exact(&mut greeting).await.unwrap();
            assert_eq!(greeting, [5, 1, 2]);
            stream.write_all(&[5, 2]).await.unwrap();
            let mut auth = [0; 5];
            stream.read_exact(&mut auth).await.unwrap();
            assert_eq!(&auth, &[1, 1, b'u', 1, b'p']);
            stream.write_all(&[1, 0]).await.unwrap();
            let mut connect = [0; 5];
            stream.read_exact(&mut connect).await.unwrap();
            assert_eq!(connect, [5, 1, 0, 3, 19]);
            let mut target = [0; 21];
            stream.read_exact(&mut target).await.unwrap();
            assert_eq!(&target[..19], b"member.bilibili.com");
            assert_eq!(&target[19..], &443_u16.to_be_bytes());
            stream
                .write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 0])
                .await
                .unwrap();
            stream.write_all(b"ok").await.unwrap();
        });
        let proxy = VerificationProxy::start(&format!("socks5://u:p@{address}"))
            .await
            .unwrap();
        let mut browser = browser(&proxy).await;
        assert!(
            read_headers(&mut browser)
                .await
                .unwrap()
                .starts_with("HTTP/1.1 200")
        );
        let mut received = [0; 2];
        browser.read_exact(&mut received).await.unwrap();
        assert_eq!(&received, b"ok");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn rejected_authentication_does_not_establish_a_tunnel() {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = upstream.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = upstream.accept().await.unwrap();
            read_headers(&mut stream).await.unwrap();
            stream
                .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
                .await
                .unwrap();
        });
        let proxy = VerificationProxy::start(&format!("http://{address}"))
            .await
            .unwrap();
        let mut browser = browser(&proxy).await;
        assert!(
            read_headers(&mut browser)
                .await
                .unwrap()
                .starts_with("HTTP/1.1 502")
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn https_proxy_uses_tls_before_sending_credentials() {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = upstream.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = upstream.accept().await.unwrap();
            let mut record = [0; 3];
            stream.read_exact(&mut record).await.unwrap();
            assert_eq!(record[0], 0x16); // TLS 握手，而非明文 CONNECT 或密码。
            assert_eq!(record[1], 3);
        });
        let proxy = VerificationProxy::start(&format!("https://u:p@{address}"))
            .await
            .unwrap();
        let mut browser = browser(&proxy).await;
        // 上游未完成 TLS 握手时，不能向 Webview 宣告隧道已连接。
        assert!(
            read_headers(&mut browser)
                .await
                .unwrap()
                .starts_with("HTTP/1.1 502")
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn dropping_proxy_closes_active_tunnels() {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy = VerificationProxy::start(&format!("http://{}", upstream.local_addr().unwrap()))
            .await
            .unwrap();
        let mut browser = browser(&proxy).await;
        let (mut stream, _) = upstream.accept().await.unwrap();
        read_headers(&mut stream).await.unwrap();
        stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await.unwrap();
        read_headers(&mut browser).await.unwrap();
        drop(proxy);
        let mut byte = [0];
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), browser.read(&mut byte))
                .await
                .unwrap()
                .unwrap(),
            0
        );
    }
}
