// src/core/services/notice.service.ts

import { noticeApi } from '@lib/client';

import { applyFilters, sortByDateDesc } from '@lib/utils/filters';
import { calculateStats, getLatestDate } from '@lib/utils/stats';

import type { ApiResponse } from '@/types/api';
import type { FilterOptions } from '@/types/filter';
import type { Notice, NoticeStats, ArchivePeriod } from '@/types/notice';

/**
 * Notice business logic service
 */
export class NoticeService {
  /**
   * Get latest notices
   */
  async getCurrentNotices(): Promise<ApiResponse<Notice[]>> {
    return await noticeApi.fetchCurrent();
  }

  /**
   * Get notices for a specific month
   */
  async getNoticesByMonth(year: number, month: number): Promise<ApiResponse<Notice[]>> {
    return await noticeApi.fetchByMonth(year, month);
  }

  /**
   * Get notices for multiple months
   */
  async getNoticesByMonths(periods: ArchivePeriod[]): Promise<ApiResponse<Notice[]>> {
    return await noticeApi.fetchByMonths(periods);
  }

  /**
   * Get filtered notices
   */
  filterNotices(notices: Notice[], options: FilterOptions): Notice[] {
    return applyFilters(notices, options);
  }

  /**
   * Get sorted notices
   */
  sortNotices(notices: Notice[]): Notice[] {
    return sortByDateDesc(notices);
  }

  /**
   * Calculate statistics
   */
  calculateStatistics(notices: Notice[]): NoticeStats {
    return calculateStats(notices);
  }

  /**
   * Get latest notice date
   */
  getLatestNoticeDate(notices: Notice[]): string {
    return getLatestDate(notices);
  }

  /**
   * Filter and sort notices
   */
  processNotices(notices: Notice[], options: FilterOptions): Notice[] {
    const filtered = this.filterNotices(notices, options);
    return this.sortNotices(filtered);
  }
}

/**
 * Default NoticeService Instance
 */
export const noticeService = new NoticeService();
