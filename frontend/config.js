// Configuration for frontend
const CONFIG = {
    API_BASE_URL: window.location.hostname === 'localhost'
        ? 'http://localhost:3000/api'
        : (window.FOSSDB_API_URL || '/api'),
};

// Make available globally
window.FOSSDB_CONFIG = CONFIG;
