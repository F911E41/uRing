// src/lib/client.ts

import type { ApiResponse } from '@/types/api';
import type { Notice, ArchivePeriod } from '@/types/notice';

// @TODO: Move to environment variable
const API_BASE_PATH = '/v1';

/**
 * uRing API Client
 */
export class NoticeApiClient {
    private basePath: string;

    constructor(basePath: string = API_BASE_PATH) {
        this.basePath = basePath;
    }

    /**
     * Fetch JSON data (common method)
     */
    private async fetchJson<T>(path: string): Promise<ApiResponse<T>> {
        try {
            const response = await fetch(`${this.basePath}${path}`);

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            const data = await response.json();

            return {
                status: 'success',
                data,
            };
        } catch (error) {
            console.error(`Failed to fetch ${path}:`, error);
            return {
                status: 'error',
                data: [] as T,
                message: error instanceof Error ? error.message : 'Unknown error',
            };
        }
    }

    /**
     * Fetch current notices
     */
    async fetchCurrent(): Promise<ApiResponse<Notice[]>> {
        const response = await this.fetchJson<Notice[]>('/current.json');

        if (response.status === 'success') {
            return {
                ...response,
                data: Array.isArray(response.data) ? response.data : [],
            };
        }

        return response;
    }

    /**
     * Fetch notices for a specific month
     */
    async fetchByMonth(year: number, month: number): Promise<ApiResponse<Notice[]>> {
        const monthStr = month.toString().padStart(2, '0');
        const response = await this.fetchJson<Notice[]>(`/stacks/${year}/${monthStr}.json`);

        if (response.status === 'success') {
            return {
                ...response,
                data: Array.isArray(response.data) ? response.data : [],
            };
        }

        return response;
    }

    /**
     * Fetch notices for multiple months
     */
    async fetchByMonths(periods: ArchivePeriod[]): Promise<ApiResponse<Notice[]>> {
        try {
            const results = await Promise.all(
                periods.map((period) => this.fetchByMonth(period.year, period.month))
            );

            const allNotices = results.flatMap((result) =>
                result.status === 'success' ? result.data : []
            );

            // Sort by date (latest first)
            allNotices.sort((a, b) => (b.date || '').localeCompare(a.date || ''));

            return {
                status: 'success',
                data: allNotices,
            };
        } catch (error) {
            console.error('Failed to fetch notices for multiple months:', error);
            return {
                status: 'error',
                data: [],
                message: error instanceof Error ? error.message : 'Unknown error',
            };
        }
    }
}

/**
 * Default API client instance
 */
export const noticeApi = new NoticeApiClient();
