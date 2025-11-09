// Theme toggle functionality
(function() {
    const THEME_KEY = 'rune-theme';

    // Get saved theme or null
    function getSavedTheme() {
        return localStorage.getItem(THEME_KEY);
    }

    // Save theme to localStorage
    function saveTheme(theme) {
        if (theme === 'light' || theme === 'dark') {
            localStorage.setItem(THEME_KEY, theme);
        }
    }

    // Apply theme to body
    function applyTheme(theme) {
        document.body.classList.remove('light-theme', 'dark-theme');
        if (theme === 'light') {
            document.body.classList.add('light-theme');
        } else if (theme === 'dark') {
            document.body.classList.add('dark-theme');
        }
        // If theme is null, don't add any class - let CSS media query handle it
    }

    // Toggle theme
    function toggleTheme() {
        const currentTheme = getSavedTheme();
        const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;

        let newTheme;
        if (currentTheme === 'light') {
            newTheme = 'dark';
        } else if (currentTheme === 'dark') {
            newTheme = prefersDark ? null : 'light';
        } else {
            // No saved theme, toggle from system preference
            newTheme = prefersDark ? 'light' : 'dark';
        }

        if (newTheme === null) {
            localStorage.removeItem(THEME_KEY);
        } else {
            saveTheme(newTheme);
        }

        applyTheme(newTheme);
    }

    // Initialize theme on page load
    function initTheme() {
        const savedTheme = getSavedTheme();
        applyTheme(savedTheme);

        // Add event listener to theme toggle button
        const toggleButton = document.querySelector('.theme-toggle');
        if (toggleButton) {
            toggleButton.addEventListener('click', toggleTheme);
        }
    }

    // Run on DOMContentLoaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initTheme);
    } else {
        initTheme();
    }
})();
