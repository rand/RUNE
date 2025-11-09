// RUNE-specific sidebar comments
// Load this BEFORE sidebar-base.js

(function() {
    // Set theme key for RUNE
    window.THEME_KEY = 'rune-theme';

    // Detect which page we're on
    const path = window.location.pathname;
    const isWhitepaper = path.includes('whitepaper');

    if (isWhitepaper) {
        // Whitepaper page comments
        window.SIDEBAR_COMMENTS = {
            'abstract': '// The agent authorization challenge',
            'table-of-contents': '// Whitepaper structure',
            '1-introduction': '// Problem space & RUNE approach',
            '2-background-and-motivation': '// Why existing solutions fall short',
            '3-system-design': '// Core concepts & architecture',
            '4-architecture': '// Dual-engine implementation',
            '5-implementation': '// Technology stack & details',
            '6-performance-evaluation': '// Benchmarks: 5M+ ops/sec',
            '7-workflows-and-use-cases': '// Production scenarios',
            '8-lessons-learned': '// Design insights & tradeoffs',
            '9-related-work': '// Comparison to alternatives',
            '10-future-work': '// Roadmap & planned features',
            '11-conclusion': '// Summary & next steps'
        };

        window.SIDEBAR_SUBSECTIONS = {};
        window.SIDEBAR_DEFAULT = '// Technical whitepaper';
    } else {
        // Index page comments
        window.SIDEBAR_COMMENTS = {
            'abstract': '// Autonomous agents need safe boundaries',
            'key-features': '// <1ms latency • 5M+ ops/sec',
            'architecture': '// Dual-engine: Datalog + Cedar',
            'use-cases': '// Production workflows & integration',
            'getting-started': '// Quick start guide'
        };

        window.SIDEBAR_SUBSECTIONS = {};
        window.SIDEBAR_DEFAULT = '// High-Performance Authorization';
    }
})();
