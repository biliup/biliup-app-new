import assert from 'node:assert/strict'
import { afterEach, test } from 'node:test'
import { JSDOM } from 'jsdom'
import { Editor } from '@tiptap/vue-3'
import StarterKit from '@tiptap/starter-kit'
import Mention from '@tiptap/extension-mention'
import { parseDescriptionClipboardText } from '../src/utils/descriptionClipboard.ts'

const { window } = new JSDOM('<!doctype html><html><body></body></html>', {
    pretendToBeVisual: true
})
for (const name of [
    'window',
    'document',
    'navigator',
    'Node',
    'HTMLElement',
    'MutationObserver',
    'getComputedStyle',
    'requestAnimationFrame',
    'cancelAnimationFrame'
]) {
    Object.defineProperty(globalThis, name, {
        configurable: true,
        value: name === 'window' ? window : window[name]
    })
}

const editors = []
const createEditor = (options = {}) => {
    const editor = new Editor({
        element: document.createElement('div'),
        extensions: [StarterKit, Mention],
        editorProps: { clipboardTextParser: parseDescriptionClipboardText },
        ...options
    })
    editors.push(editor)
    return editor
}
const paste = (editor, text) => editor.view.pasteText(text, new window.Event('paste'))
const read = editor => editor.getText({ blockSeparator: '\n' })
afterEach(() => editors.splice(0).forEach(editor => editor.destroy()))

for (const text of [
    '1\n\n\n\n1',
    '\n\n首尾\n\n',
    '\n\n\n',
    '甲\r\n\r\n乙',
    '甲\r\r乙',
    '甲\n \t \n乙',
    '<b>纯文本</b>\n\n乙'
]) {
    test(`preserves plain text ${JSON.stringify(text)}`, () => {
        const editor = createEditor()
        paste(editor, text)
        const expected = text.replace(/\r\n?/g, '\n')
        assert.equal(read(editor), expected)
        const restored = createEditor({ content: editor.getJSON() })
        assert.equal(read(restored), expected)
    })
}

test('inserts into the middle of a paragraph without losing surrounding text', () => {
    const editor = createEditor({ content: '<p>前后</p>' })
    editor.commands.setTextSelection(2)
    paste(editor, '甲\n\n乙')
    assert.equal(read(editor), '前甲\n\n乙后')
    editor.commands.undo()
    assert.equal(read(editor), '前后')
    editor.commands.redo()
    assert.equal(read(editor), '前甲\n\n乙后')
})

test('replaces a selection spanning paragraphs', () => {
    const editor = createEditor({ content: '<p>前甲</p><p>乙后</p>' })
    editor.commands.setTextSelection({ from: 2, to: 6 })
    paste(editor, '新\n\n内容')
    assert.equal(read(editor), '前新\n\n内容后')
})

test('retains active text marks', () => {
    const editor = createEditor({ content: '<p><strong>前后</strong></p>' })
    editor.commands.setTextSelection(2)
    paste(editor, '甲\n\n乙')
    assert.equal(read(editor), '前甲\n\n乙后')
    const paragraphs = editor.getJSON().content
    assert.equal(paragraphs[0].content[0].marks[0].type, 'bold')
    assert.equal(paragraphs[2].content[0].marks[0].type, 'bold')
})

test('keeps HTML paste and mentions on their existing path', () => {
    const editor = createEditor()
    editor.view.pasteHTML(
        '<p>甲</p><p></p><p><span data-type="mention" data-id="123" data-label="用户"></span></p>',
        new window.Event('paste')
    )
    assert.equal(read(editor), '甲\n\n@用户')
    assert.equal(editor.getJSON().content[2].content[0].attrs.id, '123')
})

test('does not change the existing code-block paste path', () => {
    const editor = createEditor({ content: '<pre><code></code></pre>' })
    paste(editor, '甲\n\n乙')
    assert.equal(editor.state.doc.firstChild.textContent, '甲\n\n乙')
    assert.equal(editor.getJSON().content[0].type, 'codeBlock')
})
