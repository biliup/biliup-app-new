import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import vm from 'node:vm'

const template = readFileSync(new URL('../src-tauri/src/services/archive_delete_verification.js', import.meta.url), 'utf8')
const validation = { geetest_challenge: 'sdk-challenge', geetest_validate: 'sdk-validate', geetest_seccode: 'independent-sdk-seccode' }
const settle = async condition => {
    for (let i = 0; i < 40 && !condition(); i++) await Promise.resolve()
    assert.ok(condition())
}

function run({ sms = false, uid = 42, origin = 'https://member.bilibili.com', proof = validation, checkFails = false } = {}) {
    const catalog = JSON.parse(readFileSync(new URL('../src-tauri/src/i18n/zh-CN.json', import.meta.url), 'utf8'))
    const messages = Object.fromEntries(Object.entries(catalog).filter(([key]) => key.startsWith('delete.script.')).map(([key, value]) => [key.slice('delete.script.'.length), value]))
    Object.assign(messages, { origin: '来源错误', account_mismatch: '账号错误', token_invalid: '凭据错误' })
    class Element {
        constructor(tag) { this.tag = tag; this.children = []; this.textContent = ''; this.value = '' }
        append(...nodes) { this.children.push(...nodes) }
        appendChild(node) { this.children.push(node); return node }
        replaceChildren(...nodes) { this.children = nodes }
    }
    const document = { head: new Element('head'), body: new Element('body'), cookie: 'bili_jct=test-csrf', createElement: tag => new Element(tag) }
    const calls = [], navigations = [], timers = new Map()
    let success, captchaOptions
    const window = { initGeetest: (options, callback) => {
        captchaOptions = options
        callback({ appendTo: () => {}, onError: () => {}, onSuccess: fn => { success = fn }, getValidate: () => proof })
    } }
    const source = template.replace('__RISK_MESSAGES__', JSON.stringify(messages))
        .replace('__RISK_OPTIONS__', JSON.stringify({ smsRequired: sms }))
        .replace('__RISK_READY_CALLBACK__', JSON.stringify('https://member.bilibili.com/__biliup_upload_verification__#delete-session&ready=1'))
        .replace('__EXPECTED_UID__', '42')
    const promise = vm.runInNewContext(source, {
        window, document, location: { origin, set href(value) { navigations.push(value) } },
        AbortController, URLSearchParams, setTimeout, clearTimeout,
        setInterval: fn => { const id = timers.size + 1; timers.set(id, fn); return id },
        clearInterval: id => timers.delete(id),
        fetch: async (url, options) => {
            calls.push({ url, options })
            let data
            if (url.endsWith('/nav')) data = { mid: uid }
            else if (url.endsWith('/hide/tel')) data = { tel: '138****0000' }
            else if (url.endsWith('/sms/send')) data = { token: 'sms-token' }
            else if (url.endsWith('/sms/check')) {
                if (checkFails) return { ok: true, json: async () => ({ code: -1, message: '短信错误' }) }
            } else if (url.startsWith('/x/geetest/pre')) data = { gt: 'gt', challenge: 'initial-challenge', success: 1, new_captcha: 1 }
            else assert.fail(`不应发起请求: ${url}`)
            return { ok: true, json: async () => ({ code: 0, data }) }
        }
    })
    const elements = () => {
        const all = []
        const visit = node => { all.push(node); node.children.forEach(visit) }
        visit(document.body)
        return all
    }
    return { promise, calls, navigations, timers, elements, messages, get success() { return success }, get captchaOptions() { return captchaOptions } }
}

test('无需短信时使用官方极验结果，脚本本身不提交删除', async () => {
    const h = run()
    await settle(() => Boolean(h.success))
    assert.equal(h.captchaOptions.challenge, 'initial-challenge')
    assert.equal(h.captchaOptions.product, 'popup')
    assert.equal(h.captchaOptions.offline, false)
    h.success()
    const result = JSON.parse(await h.promise)
    assert.deepEqual(result, { ...validation, success: 1 })
    assert.equal(h.calls.length, 2)
    assert.ok(h.navigations[0].endsWith('&ready=1'))
    assert.equal(h.calls.some(call => call.url.includes('/archive/delete')), false)
})

test('短信由用户发送并校验，完成后才启动极验并携带短信结果', async () => {
    const h = run({ sms: true })
    await settle(() => h.elements().some(node => node.textContent === h.messages.send_sms))
    assert.equal(h.calls.some(call => call.url.endsWith('/sms/send')), false)
    assert.equal(h.success, undefined)
    const send = h.elements().find(node => node.textContent === h.messages.send_sms)
    const confirm = h.elements().find(node => node.textContent === h.messages.confirm_sms)
    const input = h.elements().find(node => node.tag === 'input')
    await send.onclick()
    assert.equal(new URLSearchParams(h.calls.at(-1).options.body).get('csrf'), 'test-csrf')
    assert.equal(send.disabled, true)
    input.value = 'bad'
    await confirm.onclick()
    assert.equal(h.calls.some(call => call.url.endsWith('/sms/check')), false)
    input.value = '123456'
    await confirm.onclick()
    await settle(() => Boolean(h.success))
    const check = h.calls.find(call => call.url.endsWith('/sms/check'))
    assert.equal(new URLSearchParams(check.options.body).get('token'), 'sms-token')
    assert.equal(new URLSearchParams(check.options.body).get('code'), '123456')
    h.success()
    const result = JSON.parse(await h.promise)
    assert.equal(result.sms_code, '123456')
    assert.equal(h.timers.size, 0)
})

test('短信校验失败可以重试，不能进入极验或提交删除', async () => {
    const h = run({ sms: true, checkFails: true })
    await settle(() => h.elements().some(node => node.textContent === h.messages.send_sms))
    const send = h.elements().find(node => node.textContent === h.messages.send_sms)
    const confirm = h.elements().find(node => node.textContent === h.messages.confirm_sms)
    await send.onclick()
    h.elements().find(node => node.tag === 'input').value = '123456'
    await confirm.onclick()
    assert.equal(confirm.disabled, false)
    assert.equal(h.success, undefined)
    assert.ok(h.elements().some(node => node.textContent === '短信错误'))
})

test('来源、账号或验证码结果无效时拒绝，取消按钮只回传取消', async () => {
    await assert.rejects(run({ origin: 'https://example.com' }).promise, /来源错误/)
    await assert.rejects(run({ uid: 99 }).promise, /账号错误/)
    const h = run({ proof: { geetest_validate: 'incomplete' } })
    await settle(() => Boolean(h.success))
    h.success()
    await assert.rejects(h.promise, /凭据错误/)
    const cancel = run()
    await settle(() => Boolean(cancel.success))
    cancel.elements().find(node => node.textContent === cancel.messages.cancel).onclick()
    assert.ok(cancel.navigations.at(-1).endsWith('&cancelled=1'))
})
