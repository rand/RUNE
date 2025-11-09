// Dynamic sidebar content based on scroll position
(function() {
    // Section-specific comments
    const sectionComments = {
        'abstract': '// Autonomous agents need safe boundaries',
        'key-features': '// <1ms latency • 5M+ ops/sec',
        'architecture': '// Dual-engine: Datalog + Cedar',
        'use-cases': '// Production workflows & integration',
        'getting-started': '// Quick start guide'
    };

    function updateSidebarContent() {
        const sidebar = document.querySelector('.sidebar-tagline');
        if (!sidebar) return;

        // Get all sections
        const sections = document.querySelectorAll('section[id]');
        const scrollPosition = window.scrollY + window.innerHeight / 3;

        // Find the current section
        let currentSection = null;
        sections.forEach(section => {
            const top = section.offsetTop;
            const bottom = top + section.offsetHeight;

            if (scrollPosition >= top && scrollPosition <= bottom) {
                currentSection = section.id;
            }
        });

        // Update sidebar content
        if (currentSection && sectionComments[currentSection]) {
            sidebar.textContent = sectionComments[currentSection];
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
