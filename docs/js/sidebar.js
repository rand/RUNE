// Dynamic sidebar content based on scroll position
(function() {
    // Section-specific comments for index page
    const indexComments = {
        'abstract': '// Autonomous agents need safe boundaries',
        'key-features': '// <1ms latency • 5M+ ops/sec',
        'architecture': '// Dual-engine: Datalog + Cedar',
        'use-cases': '// Production workflows & integration',
        'getting-started': '// Quick start guide'
    };

    // Section-specific comments for whitepaper
    const whitepaperComments = {
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

    // Detect which page we're on and use appropriate comments
    function getSectionComments() {
        if (window.location.pathname.includes('whitepaper')) {
            return whitepaperComments;
        } else if (window.location.pathname.includes('agent-guide')) {
            return {}; // Agent guide uses static message
        }
        return indexComments;
    }

    function updateSidebarContent() {
        const sidebar = document.querySelector('.sidebar-tagline');
        if (!sidebar) return;

        const sectionComments = getSectionComments();

        // Get all sections (both section[id] and h2[id])
        const sections = [...document.querySelectorAll('section[id], h2[id]')];

        // Account for navbar height and use a better scroll threshold
        const navbarHeight = 80; // navbar + some buffer
        const scrollPosition = window.scrollY + navbarHeight + 100;

        // Build section boundaries by finding distance to next heading
        const sectionBoundaries = sections.map((element, index) => {
            const top = element.offsetTop;
            const nextElement = sections[index + 1];
            const bottom = nextElement ? nextElement.offsetTop : document.body.scrollHeight;

            return {
                id: element.id,
                top: top,
                bottom: bottom
            };
        });

        // Find the current section
        let currentSection = null;
        for (const section of sectionBoundaries) {
            if (scrollPosition >= section.top && scrollPosition < section.bottom) {
                currentSection = section.id;
                break;
            }
        }

        // Update sidebar content
        if (currentSection && sectionComments[currentSection]) {
            sidebar.textContent = sectionComments[currentSection];
        } else if (Object.keys(sectionComments).length === 0) {
            // Keep static message for pages without dynamic content
            return;
        } else {
            sidebar.textContent = '// High-Performance Authorization';
        }
    }

    // Initialize on page load
    function init() {
        updateSidebarContent();

        // Update on scroll with throttling
        let ticking = false;
        window.addEventListener('scroll', function() {
            if (!ticking) {
                window.requestAnimationFrame(function() {
                    updateSidebarContent();
                    ticking = false;
                });
                ticking = true;
            }
        });
    }

    // Run on DOMContentLoaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
