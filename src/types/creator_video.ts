export interface CreatorVideo {
    aid: number
    bvid: string
    title: string
    season_add_state?: number | null
    cover: string // 后端下载并编码的 Base64 图片，空字符串表示无封面或下载失败
}

export interface CreatorVideoPage {
    items: CreatorVideo[]
    total: number
}
