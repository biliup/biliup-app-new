import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import vm from 'node:vm'

const template = readFileSync(
    new URL('../src-tauri/src/services/upload_risk_verification.js', import.meta.url),
    'utf8'
)

function run({ sdk, mid = 42, origin = 'https://member.bilibili.com', document = {} } = {}) {
    const source = template
        .replace(
            '__RISK_OPTIONS__',
            JSON.stringify({ riskParams: { v_voucher: 'voucher";evil()' } })
        )
        .replace('__EXPECTED_UID__', '42')
    return vm.runInNewContext(source, {
        location: { origin },
        window: sdk ? { CaptchaLoader: { load: async () => sdk } } : {},
        document,
        fetch: async () => ({ json: async () => ({ code: 0, data: { mid } }) }),
        setTimeout,
        clearTimeout
    })
}

test('验证账号后等待 SDK Promise 完成，凭据作为字符串返回', async () => {
    assert.equal(
        await run({
            sdk: async options => {
                assert.equal(options.riskParams.v_voucher, 'voucher";evil()')
                await Promise.resolve()
                return 'sdk-token'
            }
        }),
        'sdk-token'
    )
})

test('账号或来源不一致时不能启动验证码', async () => {
    const sdk = async () => assert.fail('不应调用 SDK')
    await assert.rejects(run({ sdk, mid: 99 }), /账号.*不一致/)
    await assert.rejects(run({ sdk, origin: 'https://tauri.localhost' }), /B 站投稿页面/)
})

test('取消和无效验证结果不会被判定为成功', async () => {
    const cancelled = new Error('用户取消')
    await assert.rejects(
        run({
            sdk: async () => {
                throw cancelled
            }
        }),
        error => error === cancelled
    )
    for (const token of ['', ' ', null, {}]) {
        await assert.rejects(run({ sdk: async () => token }), /未返回有效凭据/)
    }
})

test('SDK 加载失败明确报错', async () => {
    await assert.rejects(
        run({
            document: {
                createElement: () => ({ remove() {} }),
                head: {
                    appendChild(script) {
                        assert.equal(
                            script.src,
                            'https://s1.hdslb.com/bfs/seed/jinkela/risk-captcha-sdk/CaptchaLoader.js'
                        )
                        script.onerror()
                    }
                }
            }
        }),
        /加载失败/
    )
})
