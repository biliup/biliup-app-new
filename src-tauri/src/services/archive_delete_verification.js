(async () => {
    const messages = __RISK_MESSAGES__
    const options = __RISK_OPTIONS__
    const readyCallback = __RISK_READY_CALLBACK__
    window.__biliupRiskStage = 'origin'
    if (location.origin !== 'https://member.bilibili.com') throw new Error(messages.origin)

    const request = async (url, data) => {
        const controller = new AbortController()
        const timer = setTimeout(() => controller.abort(), 30000)
        try {
            const response = await fetch(url, {
                credentials: 'include',
                signal: controller.signal,
                ...(data ? {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
                    body: new URLSearchParams(data).toString()
                } : {})
            })
            if (!response.ok) throw new Error(`${messages.failed} (${response.status})`)
            const result = await response.json()
            if (result.code !== 0) throw new Error(result.message || result.msg || messages.failed)
            return result.data
        } finally {
            clearTimeout(timer)
        }
    }

    window.__biliupRiskStage = 'account'
    const account = await request('https://api.bilibili.com/x/web-interface/nav')
    if (account?.mid !== __EXPECTED_UID__) throw new Error(messages.account_mismatch)

    const style = document.createElement('style')
    style.textContent = 'body{margin:0;background:#f5f7fa;font:16px system-ui;color:#303133} .delete-panel{max-width:520px;margin:60px auto;padding:28px;background:white;border-radius:12px} .delete-panel button,.delete-panel input{font:inherit;padding:10px;margin:8px 8px 8px 0} .delete-panel p{line-height:1.6} .delete-error{color:#f56c6c;white-space:pre-wrap}'
    document.head.appendChild(style)
    const panel = document.createElement('main')
    panel.className = 'delete-panel'
    const heading = document.createElement('h2')
    heading.textContent = messages.title
    const warning = document.createElement('p')
    warning.textContent = messages.warning
    const content = document.createElement('div')
    const status = document.createElement('p')
    status.className = 'delete-error'
    const cancel = document.createElement('button')
    cancel.textContent = messages.cancel
    cancel.onclick = () => { location.href = readyCallback.replace('&ready=1', '&cancelled=1') }
    panel.append(heading, warning, content, status, cancel)
    document.body.replaceChildren(panel)

    window.__biliupRiskStage = 'sdk_load'
    if (typeof window.initGeetest !== 'function') {
        await new Promise((resolve, reject) => {
            const script = document.createElement('script')
            const timer = setTimeout(() => reject(new Error(messages.sdk_timeout)), 30000)
            script.src = 'https://static.geetest.com/static/tools/gt.js'
            script.onload = () => { clearTimeout(timer); resolve() }
            script.onerror = () => { clearTimeout(timer); reject(new Error(messages.sdk_load_failed)) }
            document.head.appendChild(script)
        })
    }
    // SDK 就绪后停止窗口初始化计时，短信与人工验证不设总超时。
    location.href = readyCallback
    window.__biliupRiskStage = 'sdk_verify'
    let smsCode
    if (options.smsRequired) {
        const phone = await request('https://api.bilibili.com/x/silverbullet-proxy/web/recaptcha/hide/tel')
        const csrfCookie = document.cookie.split(';').map(value => value.trim()).find(value => value.startsWith('bili_jct='))
        if (!csrfCookie) throw new Error(messages.sms_failed)
        const csrf = decodeURIComponent(csrfCookie.slice('bili_jct='.length))
        smsCode = await new Promise(resolve => {
            let token, countdown, remaining = 0, checking = false
            const phoneLabel = document.createElement('p')
            phoneLabel.textContent = phone?.tel || ''
            const input = document.createElement('input')
            input.placeholder = messages.sms_placeholder
            input.inputMode = 'numeric'
            input.maxLength = 6
            const send = document.createElement('button')
            send.textContent = messages.send_sms
            const confirm = document.createElement('button')
            confirm.textContent = messages.confirm_sms
            confirm.disabled = true
            content.append(phoneLabel, input, send, confirm)
            send.onclick = async () => {
                if (checking) return
                send.disabled = true
                confirm.disabled = true
                token = undefined
                status.textContent = ''
                try {
                    const data = await request('https://api.bilibili.com/x/silverbullet-proxy/web/recaptcha/sms/send', { csrf })
                    if (typeof data?.token !== 'string' || !data.token) throw new Error(messages.sms_failed)
                    token = data.token
                    confirm.disabled = false
                    status.textContent = messages.sms_sent
                    remaining = 60
                    send.textContent = `${messages.send_sms} (${remaining})`
                    countdown = setInterval(() => {
                        remaining -= 1
                        send.textContent = remaining > 0 ? `${messages.send_sms} (${remaining})` : messages.send_sms
                        if (remaining <= 0) { clearInterval(countdown); send.disabled = checking }
                    }, 1000)
                } catch (error) {
                    status.textContent = error.message || messages.sms_failed
                    send.disabled = false
                }
            }
            confirm.onclick = async () => {
                const code = input.value.trim()
                if (!token || !/^\d{6}$/.test(code)) { status.textContent = messages.sms_invalid; return }
                confirm.disabled = true
                checking = true
                send.disabled = true
                try {
                    await request('https://api.bilibili.com/x/silverbullet-proxy/web/recaptcha/sms/check', { token, code, csrf })
                    clearInterval(countdown)
                    resolve(code)
                } catch (error) {
                    status.textContent = error.message || messages.sms_invalid
                    confirm.disabled = false
                    checking = false
                    send.disabled = remaining > 0
                }
            }
        })
    }

    window.__biliupRiskStage = 'sdk_verify'
    content.replaceChildren()
    status.textContent = ''
    const label = document.createElement('p')
    label.textContent = messages.captcha
    const container = document.createElement('div')
    content.append(label, container)
    const data = await request('/x/geetest/pre?t=' + Date.now())
    if (!data?.gt || !data.challenge || ![0, 1].includes(data.success)) throw new Error(messages.token_invalid)
    return await new Promise((resolve, reject) => {
        window.initGeetest({
            gt: data.gt,
            challenge: data.challenge,
            new_captcha: data.new_captcha,
            product: 'popup',
            offline: !data.success,
            lang: messages.language
        }, captcha => {
            captcha.appendTo(container)
            captcha.onError(() => { status.textContent = messages.failed })
            captcha.onSuccess(() => {
                const validation = captcha.getValidate()
                if (!validation || !['geetest_challenge', 'geetest_validate', 'geetest_seccode'].every(key => typeof validation[key] === 'string' && validation[key].trim())) {
                    reject(new Error(messages.token_invalid))
                    return
                }
                window.__biliupRiskStage = 'sdk_result'
                resolve(JSON.stringify({
                    geetest_challenge: validation.geetest_challenge,
                    geetest_validate: validation.geetest_validate,
                    geetest_seccode: validation.geetest_seccode,
                    success: data.success,
                    sms_code: smsCode
                }))
            })
        })
    })
})()
