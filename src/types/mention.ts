export interface MentionUserItem {
    face: string
    fans: number
    name: string
    official_verify_type: number
    uid: string
}

export interface MentionUserGroup {
    group_name: string
    group_type: number
    items: MentionUserItem[]
}

export interface MentionOption {
    value: string
    uid: string
    name: string
    face: string
    avatarSrc: string
    fans: number
    groupName: string
    showGroupLabel: boolean
}
