// src/core/lib/api.ts

/**
 * API Fetcher for Crawler Results
 * Fetches notice data aggregated from the Crawler's results
 */

export interface Notice {
    campus: string;
    college?: string;
    department_id: string;
    department_name: string;
    board_id: string;
    board_name: string;
    title: string;
    date: string;
    link: string;
}

export interface ApiResponse {
    data: Notice[];
    status: 'success' | 'error';
    message?: string;
}

/**
 * Fetches all notices from the aggregated notices.json file
 * @returns Promise with API response containing all notices
 */
export async function fetchAllNotices(): Promise<ApiResponse> {
    try {
        const response = await fetch('/data/notices.json');

        if (!response.ok) {
            throw new Error('Failed to fetch notices.json');
        }

        const data: Notice[] = await response.json();

        return {
            data,
            status: 'success',
            message: `Successfully fetched ${data.length} notices`,
        };
    } catch (error) {
        const errorMessage =
            error instanceof Error ? error.message : 'Unknown error';

        return {
            data: [],
            status: 'error',
            message: `Error fetching notices: ${errorMessage}`,
        };
    }
}

/**
 * Fetches notices filtered by campus and optionally by department
 * @param campus - Campus name to filter by
 * @param department - Optional department name to filter by
 * @returns Promise with filtered API response
 */
export async function fetchNoticesByCampus(
    campus: string,
    department?: string
): Promise<ApiResponse> {
    try {
        const response = await fetchAllNotices();

        if (response.status === 'error') {
            return response;
        }

        let filtered = response.data.filter((notice) => notice.campus === campus);

        if (department) {
            filtered = filtered.filter(
                (notice) => notice.department_name === department
            );
        }

        return {
            data: filtered,
            status: 'success',
            message: `Found ${filtered.length} notices for ${campus}${department ? ` / ${department}` : ''}`,
        };
    } catch (error) {
        const errorMessage =
            error instanceof Error ? error.message : 'Unknown error';

        return {
            data: [],
            status: 'error',
            message: `Error filtering notices: ${errorMessage}`,
        };
    }
}
