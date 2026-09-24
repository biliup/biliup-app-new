import { open } from '@tauri-apps/plugin-dialog'

const COVER_EXTENSIONS = ['jpg', 'jpeg', 'png', 'pjp', 'pjpeg', 'jiff', 'gif']

type UploadCover = (uid: number, file: string) => Promise<string | undefined>

export const selectAndUploadCover = async (
    uid: number,
    uploadCover: UploadCover
): Promise<string | undefined> => {
    const selected = await open({
        multiple: false,
        filters: [
            {
                name: 'Image',
                extensions: COVER_EXTENSIONS
            }
        ]
    })

    if (!selected) {
        return undefined
    }

    const url = await uploadCover(uid, selected)
    if (!url) {
        throw new Error('封面上传失败')
    }

    return url
}
