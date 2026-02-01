// src/core/utils/stats.ts

import type { Notice, NoticeStats } from '@/types/notice';

/**
 * Calculate statistics from the list of notices
 */
export function calculateStats(notices: Notice[]): NoticeStats {
  return {
    total: notices.length,
    campuses: new Set(notices.map((n) => n.campus)).size,
    departments: new Set(notices.map((n) => n.department_name)).size,
    boards: new Set(notices.map((n) => n.board_name)).size,
    colleges: new Set(notices.map((n) => n.college)).size,
  };
}

/**
 * Get the latest date from the list of notices
 */
export function getLatestDate(notices: Notice[]): string {
  return notices.reduce((latest, notice) => {
    if (!notice.date) return latest;
    return notice.date > latest ? notice.date : latest;
  }, '');
}

/**
 * Get the date range from the list of notices
 */
export function getDateRange(notices: Notice[]): { earliest: string; latest: string } {
  const dates = notices.map((n) => n.date).filter(Boolean).sort();
  return {
    earliest: dates[0] || '',
    latest: dates[dates.length - 1] || '',
  };
}
