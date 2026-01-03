// src/core/lib/api.ts

/**
 * Dummy API Fetcher
 * Simulates API responses using local JSON data
 */

export interface Notice {
    campus: string;
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
 * Simulates fetching notices from an API
 * Currently uses local JSON files (resp1.json, resp2.json, resp3.json)
 * @param responseId - Which response file to fetch (1, 2, or 3)
 * @returns Promise with API response containing notices
 */
export async function fetchNotices(
    responseId: number = 1
): Promise<ApiResponse> {
    try {
        // Simulate network delay
        await new Promise((resolve) => setTimeout(resolve, 300));

        const validIds = [1, 2, 3];
        const id = validIds.includes(responseId) ? responseId : 1;

        const response = await fetch(`/data/resp${id}.json`);

        if (!response.ok) {
            throw new Error(`Failed to fetch resp${id}.json`);
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
 * Fetches all available notices from all data sources
 * @returns Promise with combined API response
 */
export async function fetchAllNotices(): Promise<ApiResponse> {
    try {
        const [resp1, resp2, resp3] = await Promise.all([
            fetchNotices(1),
            fetchNotices(2),
            fetchNotices(3),
        ]);

        const allData = [...resp1.data, ...resp2.data, ...resp3.data];

        return {
            data: allData,
            status: 'success',
            message: `Successfully fetched ${allData.length} notices from all sources`,
        };
    } catch (error) {
        const errorMessage =
            error instanceof Error ? error.message : 'Unknown error';

        return {
            data: [],
            status: 'error',
            message: `Error fetching all notices: ${errorMessage}`,
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
