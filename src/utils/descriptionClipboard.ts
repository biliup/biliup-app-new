import { DOMParser, DOMSerializer } from '@tiptap/pm/model'
import type { EditorProps } from '@tiptap/pm/view'

export const parseDescriptionClipboardText: NonNullable<EditorProps['clipboardTextParser']> = (
    text,
    context,
    _plainText,
    view
) => {
    const { schema } = view.state
    const serializer = DOMSerializer.fromSchema(schema)
    const container = view.dom.ownerDocument.createElement('div')

    // Unlike ProseMirror's default parser, retain a paragraph for every newline.
    text.split(/\r\n?|\n/).forEach(line => {
        const paragraph = container.appendChild(container.ownerDocument.createElement('p'))
        if (line) {
            paragraph.appendChild(
                serializer.serializeNode(schema.text(line, context.marks()), {
                    document: container.ownerDocument
                })
            )
        }
    })

    return DOMParser.fromSchema(schema).parseSlice(container, {
        preserveWhitespace: true,
        context
    })
}
