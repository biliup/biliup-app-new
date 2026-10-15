(async () => {
    const messages = __RISK_MESSAGES__
    window.__biliupRiskStage = 'origin'
    if (location.origin !== 'https://member.bilibili.com') {
        throw new Error(messages.origin)
    }
    window.__biliupRiskStage = 'account'
    const expectedUid = __EXPECTED_UID__
    console.info('[上传风控] 检查验证窗口登录账号')
    const controller = new AbortController()
    const timeout = __RISK_TIMEOUT_MS__
    const accountTimer = timeout === null ? null : setTimeout(() => controller.abort(), timeout)
    let response, user
    try {
        response = await fetch('https://api.bilibili.com/x/web-interface/nav', {
            credentials: 'include',
            signal: controller.signal
        })
        user = await response.json()
    } finally {
        if (accountTimer !== null) clearTimeout(accountTimer)
    }
    console.info('[上传风控] 账号检查', { status: response.status, code: user.code, matches: user.data?.mid === expectedUid })
    if (user.code !== 0) {
        throw new Error(messages.account_invalid + user.code)
    }
    if (user.data?.mid !== expectedUid) {
        throw new Error(messages.account_mismatch)
    }
    const options = __RISK_OPTIONS__
    window.__biliupRiskStage = 'sdk_load'
    console.info('[上传风控] 加载官方验证码 SDK')
    const loadSdk = async () => {
        if (typeof window.CaptchaLoader?.load !== 'function') {
            await new Promise((resolve, reject) => {
                const script = document.createElement('script')
                script.src = 'https://s1.hdslb.com/bfs/seed/jinkela/risk-captcha-sdk/CaptchaLoader.js'
                script.onload = resolve
                script.onerror = () => {
                    script.remove()
                    reject(new Error(messages.sdk_load_failed))
                }
                document.head.appendChild(script)
            })
        }
        return window.CaptchaLoader.load()
    }
    const useCaptcha = timeout === null ? await loadSdk() : await new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error(messages.sdk_timeout)), timeout)
        loadSdk().then(resolve, reject).finally(() => clearTimeout(timer))
    })
    // SDK 就绪后通知后端结束初始化计时，人工验证不设超时。
    location.href = __RISK_READY_CALLBACK__
    window.__biliupRiskStage = 'sdk_verify'
    console.info('[上传风控] 开始 SDK 验证')
    const token = await useCaptcha(options)
    window.__biliupRiskStage = 'sdk_result'
    if (typeof token !== 'string' || !token.trim()) {
        throw new Error(messages.token_invalid)
    }
    console.info('[上传风控] SDK 验证成功')
    return token
})()
