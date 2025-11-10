// RUNE Theme Wrapper - Uses shared foundation
// This file is a thin wrapper that delegates to theme-core.js

(function() {
    // Use the shared foundation's theme core
    if (window.themeCore) {
        window.themeCore.init();
    } else {
        console.error('theme-core.js not loaded. Make sure to include it before theme.js');
    }
})();
