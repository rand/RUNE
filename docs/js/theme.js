// Theme toggle functionality with SVG diagram switching
(function() {
    'use strict';

    const THEME_KEY = 'theme-preference';

    // Get current theme
    function getCurrentTheme() {
        return localStorage.getItem(THEME_KEY) || 'light';
    }

    // Save theme preference
    function saveTheme(theme) {
        localStorage.setItem(THEME_KEY, theme);
    }

    // Update SVG diagrams based on theme
    function updateDiagrams(theme) {
        // Update all img tags that reference SVG diagrams
        const images = document.querySelectorAll('img[src*=".svg"]');
        images.forEach(img => {
            const currentSrc = img.getAttribute('src');
            if (!currentSrc) return;

            // Extract base path by removing any existing -light or -dark suffix
            let basePath = currentSrc.replace(/-light\.svg$/, '.svg').replace(/-dark\.svg$/, '.svg');

            // Construct the new themed path
            const themedPath = basePath.replace(/\.svg$/, theme === 'dark' ? '-dark.svg' : '-light.svg');

            // Only update if the new path is different
            if (currentSrc !== themedPath) {
                img.setAttribute('src', themedPath);
            }
        });

        // Also handle picture elements if they exist
        const pictures = document.querySelectorAll('picture');
        pictures.forEach(picture => {
            const sources = picture.querySelectorAll('source');
            const img = picture.querySelector('img');

            // Update source elements to match current theme
            sources.forEach(source => {
                const media = source.getAttribute('media');
                if (media && media.includes('prefers-color-scheme')) {
                    // Disable sources that don't match current theme
                    const isDarkSource = media.includes('dark');
                    source.disabled = (theme === 'dark') ? !isDarkSource : isDarkSource;
                }
            });

            // Force picture element to re-evaluate sources
            if (img && img.src) {
                const currentSrc = img.src;
                img.src = '';
                img.src = currentSrc;
            }
        });
    }

    // Apply theme to document
    function applyTheme(theme) {
        document.body.classList.remove('light-theme', 'dark-theme');
        document.body.classList.add(`${theme}-theme`);
        updateDiagrams(theme);
    }

    // Toggle theme
    function toggleTheme() {
        const current = getCurrentTheme();
        const newTheme = current === 'light' ? 'dark' : 'light';
        saveTheme(newTheme);
        applyTheme(newTheme);
    }

    // Initialize theme
    function initTheme() {
        const savedTheme = getCurrentTheme();
        applyTheme(savedTheme);

        // Add toggle listener
        const toggleBtn = document.querySelector('.theme-toggle');
        if (toggleBtn) {
            toggleBtn.addEventListener('click', toggleTheme);
        }
    }

    // Run on load
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initTheme);
    } else {
        initTheme();
    }
})();
