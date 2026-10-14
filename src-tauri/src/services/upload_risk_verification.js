(async () => {
    window.__biliupRiskStage = 'origin'
    if (location.origin !== 'https://member.bilibili.com') {
        throw new Error('请在具有对应账号登录态的 B 站投稿页面完成验证')
    }
    window.__biliupRiskStage = 'account'
    const expectedUid = __EXPECTED_UID__
    console.info('[上传风控] 检查验证窗口登录账号')
    const response = await fetch('https://api.bilibili.com/x/web-interface/nav', { credentials: 'include' })
    const user = await response.json()
    console.info('[上传风控] 账号检查', { status: response.status, code: user.code, matches: user.data?.mid === expectedUid })
    if (user.code !== 0) {
        throw new Error('验证窗口登录态无效，账号接口返回 code=' + user.code)
    }
    if (user.data?.mid !== expectedUid) {
        throw new Error('验证窗口账号与上传账号不一致')
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
                    reject(new Error('验证码 SDK 加载失败'))
                }
                document.head.appendChild(script)
            })
        }
        return window.CaptchaLoader.load()
    }
    const useCaptcha = await new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error('验证码 SDK 加载超时')), 30000)
        loadSdk().then(resolve, reject).finally(() => clearTimeout(timer))
    })
    window.__biliupRiskStage = 'sdk_verify'
    console.info('[上传风控] 开始 SDK 验证')
    const token = await useCaptcha(options)
    window.__biliupRiskStage = 'sdk_result'
    if (typeof token !== 'string' || !token.trim()) {
        throw new Error('验证码 SDK 未返回有效凭据')
    }
    console.info('[上传风控] SDK 验证成功')
    return token
})()
