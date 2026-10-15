export interface SeasonEpisode {
    id: number
    aid: number
    title: string
    bvid?: string
    archiveTitle?: string
}

export interface SeasonSection {
    id: number
    title: string
    epCount?: number
}

export interface ManagedSeason {
    season: {
        id: number
        title: string
        no_section?: number
    }
    sections: { sections: SeasonSection[] }
}

export interface ManagedSection {
    section: SeasonSection
    episodes: SeasonEpisode[]
}
