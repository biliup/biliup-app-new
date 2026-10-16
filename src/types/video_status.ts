export interface ProcessingVideo {
    cid: number
    index: number
    xcode_state: number
}

export interface ArchiveProcessing {
    bvid: string
    page: { total: number; pn: number; ps: number }
    videos: ProcessingVideo[]
    audit: {
        aegis_state: number
        audit_process?: { archive_audit: number; business_audit: number } | null
    }
}

export interface TranscodeResolution {
    resolution: string
    status: string
    estimated_time: number
    start_time: number
    time_now: number
    progress?: number | null
    failure_reason: string
}

export interface TranscodeDetail {
    cid: number
    xcode_state: number
    xcode_tip: string
    fail_tip: string
    transcode_list?: TranscodeResolution[] | null
}
