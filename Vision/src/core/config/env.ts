// src/core/config/env.ts

export const NOTICES_URL = process.env.NEXT_PUBLIC_NOTICES_URL || '';
export const NOTICES_BASE_URL = process.env.NEXT_PUBLIC_NOTICES_BASE_URL || '';
export const S3_BASE_URL = process.env.NEXT_PUBLIC_S3_BASE_URL || '';
export const S3_PREFIX = process.env.NEXT_PUBLIC_S3_PREFIX || 'uRing';
export const S3_SITEMAP_KEY =
    process.env.NEXT_PUBLIC_S3_SITEMAP_KEY || `${S3_PREFIX}/config/sitemap.json`;
