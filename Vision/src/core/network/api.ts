// src/core/lib/api.ts

/**
 * API Fetcher for Crawler Results.
 * Supports local snapshots and S3-backed live data.
 */

import {
    NOTICES_BASE_URL,
    NOTICES_URL,
    S3_BASE_URL,
    S3_PREFIX,
    S3_SITEMAP_KEY,
} from '../config/env';

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

interface CampusIndex {
    campus: string;
}

const LOCAL_NOTICES_PATH = '/data/notices.json';

function normalizeBaseUrl(baseUrl: string) {
    return baseUrl.replace(/\/$/, '');
}

function splitKey(key: string) {
    return key.split('/').filter(Boolean);
}

function buildUrl(baseUrl: string, segments: string[] | string) {
    const normalizedBase = normalizeBaseUrl(baseUrl);
    const parts = Array.isArray(segments) ? segments : splitKey(segments);
    const encoded = parts
        .flatMap((part) => splitKey(part))
        .map((part) => encodeURIComponent(part));
    return `${normalizedBase}/${encoded.join('/')}`;
}

async function fetchJson<T>(url: string): Promise<T> {
    const response = await fetch(url);
    if (!response.ok) {
        throw new Error(`Failed to fetch ${url}`);
    }
    return response.json() as Promise<T>;
}

async function fetchFromS3(): Promise<Notice[] | null> {
    if (!S3_BASE_URL) {
        return null;
    }

    const sitemapUrl = buildUrl(S3_BASE_URL, S3_SITEMAP_KEY);
    const campuses = await fetchJson<CampusIndex[]>(sitemapUrl);

    const campusRequests = campuses.map((entry) => {
        const campusName = entry.campus?.trim();
        if (!campusName) {
            return Promise.resolve([]);
        }
        const campusKey = [
            ...splitKey(S3_PREFIX),
            campusName,
            'New',
            'notices.json',
        ];
        const campusUrl = buildUrl(S3_BASE_URL, campusKey);
        return fetchJson<Notice[]>(campusUrl);
    });

    const results = await Promise.allSettled(campusRequests);
    const merged = results.flatMap((result) =>
        result.status === 'fulfilled' ? result.value : []
    );

    const deduped = new Map<string, Notice>();
    for (const notice of merged) {
        if (!deduped.has(notice.link)) {
            deduped.set(notice.link, notice);
        }
    }

    return Array.from(deduped.values());
}

/**
 * Fetches all notices from the aggregated notices.json file
 * @returns Promise with API response containing all notices
 */
export async function fetchAllNotices(): Promise<ApiResponse> {
    try {
        let data: Notice[] | null = null;

        if (NOTICES_URL) {
            data = await fetchJson<Notice[]>(NOTICES_URL);
        } else if (S3_BASE_URL) {
            data = await fetchFromS3();
        } else {
            const baseUrl = NOTICES_BASE_URL ? normalizeBaseUrl(NOTICES_BASE_URL) : '';
            const url = baseUrl ? `${baseUrl}${LOCAL_NOTICES_PATH}` : LOCAL_NOTICES_PATH;
            data = await fetchJson<Notice[]>(url);
        }

        if (!data) {
            throw new Error('No notice data available');
        }

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
