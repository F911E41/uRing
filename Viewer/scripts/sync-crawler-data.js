#!/usr/bin/env node

/**
 * Sync Crawler Data Script
 * 
 * This script aggregates all JSON results from the Crawler's data/result folder
 * and creates a unified notices.json file in the Viewer's public/data folder.
 */

const fs = require('fs');
const path = require('path');

const CRAWLER_RESULT_PATH = path.resolve(__dirname, '../../Crawler/data/result');
const VIEWER_DATA_PATH = path.resolve(__dirname, '../public/data');

/**
 * Recursively find all JSON files in a directory
 */
function findJsonFiles(dir, files = []) {
    const items = fs.readdirSync(dir, { withFileTypes: true });
    
    for (const item of items) {
        const fullPath = path.join(dir, item.name);
        
        if (item.isDirectory()) {
            findJsonFiles(fullPath, files);
        } else if (item.isFile() && item.name.endsWith('.json')) {
            files.push(fullPath);
        }
    }
    
    return files;
}

/**
 * Normalize date format to YYYY.MM.DD
 */
function normalizeDate(dateStr) {
    if (!dateStr) return '';
    
    // Handle format like "25.09.22" -> "2025.09.22"
    const shortYearMatch = dateStr.match(/^(\d{2})\.(\d{2})\.(\d{2})$/);
    if (shortYearMatch) {
        const year = parseInt(shortYearMatch[1]) > 50 ? `19${shortYearMatch[1]}` : `20${shortYearMatch[1]}`;
        return `${year}.${shortYearMatch[2]}.${shortYearMatch[3]}`;
    }
    
    // Already in YYYY.MM.DD format
    return dateStr;
}

/**
 * Main function to sync crawler data
 */
function syncCrawlerData() {
    console.log('🔄 Syncing Crawler data to Viewer...\n');
    
    // Check if crawler result path exists
    if (!fs.existsSync(CRAWLER_RESULT_PATH)) {
        console.error(`❌ Crawler result path not found: ${CRAWLER_RESULT_PATH}`);
        process.exit(1);
    }
    
    // Find all JSON files
    const jsonFiles = findJsonFiles(CRAWLER_RESULT_PATH);
    console.log(`📁 Found ${jsonFiles.length} JSON files in Crawler results\n`);
    
    // Aggregate all notices
    const allNotices = [];
    const stats = {
        totalFiles: jsonFiles.length,
        totalNotices: 0,
        byCampus: {},
        byDepartment: {},
    };
    
    for (const filePath of jsonFiles) {
        try {
            const content = fs.readFileSync(filePath, 'utf-8');
            const notices = JSON.parse(content);
            
            if (!Array.isArray(notices)) {
                console.warn(`⚠️  Skipping non-array file: ${filePath}`);
                continue;
            }
            
            for (const notice of notices) {
                // Normalize the date format
                const normalizedNotice = {
                    ...notice,
                    date: normalizeDate(notice.date),
                };
                
                allNotices.push(normalizedNotice);
                
                // Update stats
                stats.byCampus[notice.campus] = (stats.byCampus[notice.campus] || 0) + 1;
                stats.byDepartment[notice.department_name] = (stats.byDepartment[notice.department_name] || 0) + 1;
            }
            
            stats.totalNotices += notices.length;
        } catch (error) {
            console.error(`❌ Error reading ${filePath}: ${error.message}`);
        }
    }
    
    // Sort notices by date (newest first)
    allNotices.sort((a, b) => {
        const dateA = a.date || '';
        const dateB = b.date || '';
        return dateB.localeCompare(dateA);
    });
    
    // Ensure viewer data directory exists
    if (!fs.existsSync(VIEWER_DATA_PATH)) {
        fs.mkdirSync(VIEWER_DATA_PATH, { recursive: true });
    }
    
    // Write aggregated notices
    const outputPath = path.join(VIEWER_DATA_PATH, 'notices.json');
    fs.writeFileSync(outputPath, JSON.stringify(allNotices, null, 2), 'utf-8');
    
    // Print stats
    console.log('📊 Sync Statistics:');
    console.log(`   Total files processed: ${stats.totalFiles}`);
    console.log(`   Total notices: ${stats.totalNotices}`);
    console.log(`\n📍 By Campus:`);
    for (const [campus, count] of Object.entries(stats.byCampus).sort()) {
        console.log(`   ${campus}: ${count}`);
    }
    console.log(`\n🎓 Departments: ${Object.keys(stats.byDepartment).length} total`);
    
    console.log(`\n✅ Successfully synced to: ${outputPath}`);
}

// Run the script
syncCrawlerData();
