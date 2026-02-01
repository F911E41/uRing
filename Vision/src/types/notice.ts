// src/types/notice.ts

/**
 * Notice Interface
 */
export interface Notice {
    campus: string;
    college: string;
    department_id: string;
    department_name: string;
    board_id: string;
    board_name: string;
    title: string;
    date: string;
    link: string;
}

/**
 * Notice Statistics Type
 */
export interface NoticeStats {
    total: number;
    campuses: number;
    departments: number;
    boards: number;
    colleges: number;
}

/**
 * Archive Period Type
 */
export interface ArchivePeriod {
    year: number;
    month: number;
}
