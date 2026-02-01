// src/types/api.ts

/**
 * API Response Wrapper
 */
export interface ApiResponse<T> {
    status: 'success' | 'error';
    data: T;
    message?: string;
}
