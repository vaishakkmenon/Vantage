import { DifficultyLevel, DifficultyConfig, SearchOptions } from '../types'

export const DIFFICULTY_CONFIGS: Record<DifficultyLevel, DifficultyConfig> = {
    beginner: {
        label: 'Beginner',
        description: 'Quick, tactical play',
        searchOptions: { depth: 4 },
    },
    intermediate: {
        label: 'Intermediate',
        description: 'Solid, positional play',
        searchOptions: { depth: 10 },
    },
    advanced: {
        label: 'Advanced',
        description: 'Deep, strategic play',
        searchOptions: { depth: 16 },
    },
    expert: {
        label: 'Expert',
        description: 'Maximum depth, timed',
        searchOptions: { movetime: 5000 },
    },
}