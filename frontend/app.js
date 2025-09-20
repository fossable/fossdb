// Global state
let currentUser = null;
let authToken = null;
let currentViewMode = 'grid';
let favoritesCache = new Set();
let comparisonList = [];

// Initialize app
document.addEventListener('DOMContentLoaded', function() {
    console.log('DOM loaded, initializing app...');
    console.log('showHome function available:', typeof window.showHome);
    console.log('showPackages function available:', typeof window.showPackages);
    checkAuthStatus();
    showHome();
    
    // Setup HTMX response handlers
    setupHTMXHandlers();
    
    // Setup HTMX request interceptors for auth headers
    document.body.addEventListener('htmx:configRequest', function(evt) {
        if (authToken && evt.detail.path.includes('/api/')) {
            evt.detail.headers['Authorization'] = 'Bearer ' + authToken;
        }
    });
});

function checkAuthStatus() {
    const token = localStorage.getItem('auth_token');
    const userData = localStorage.getItem('user_data');
    
    if (token && userData) {
        authToken = token;
        currentUser = JSON.parse(userData);
        updateAuthUI();
    }
}

function updateAuthUI() {
    const authButtons = document.getElementById('auth-buttons');
    const userMenu = document.getElementById('user-menu');
    
    if (currentUser) {
        authButtons.classList.add('hidden');
        userMenu.classList.remove('hidden');
        document.getElementById('username').textContent = currentUser.username;
    } else {
        authButtons.classList.remove('hidden');
        userMenu.classList.add('hidden');
    }
}

// Update active navigation state
function updateActiveNav(activePageId) {
    // Remove active class from all nav links
    document.querySelectorAll('.nav-link').forEach(link => {
        link.classList.remove('active');
    });
    
    // Add active class to current page nav links
    const desktopNav = document.getElementById(`nav-${activePageId}`);
    const mobileNav = document.getElementById(`mobile-nav-${activePageId}`);
    
    if (desktopNav) desktopNav.classList.add('active');
    if (mobileNav) mobileNav.classList.add('active');
}

// Navigation functions - make them globally available
window.showHome = function() {
    console.log('showHome called');
    hideAllPages();
    document.getElementById('home-page').classList.remove('hidden');
    updateActiveNav('home');
    
    // Trigger scroll to top animation
    window.scrollTo({ top: 0, behavior: 'smooth' });
};

window.showPackages = function() {
    console.log('showPackages called');
    hideAllPages();
    document.getElementById('packages-page').classList.remove('hidden');
    updateActiveNav('packages');
    
    // Trigger scroll to top animation
    window.scrollTo({ top: 0, behavior: 'smooth' });
};

window.showAnalytics = function() {
    console.log('showAnalytics called');
    hideAllPages();
    document.getElementById('analytics-page').classList.remove('hidden');
    updateActiveNav('analytics');
    
    // Trigger scroll to top animation
    window.scrollTo({ top: 0, behavior: 'smooth' });
    
    // Initialize analytics data
    loadAnalyticsData();
};

window.showAPI = function() {
    console.log('showAPI called');
    hideAllPages();
    document.getElementById('api-page').classList.remove('hidden');
    updateActiveNav('api');
    
    // Trigger scroll to top animation
    window.scrollTo({ top: 0, behavior: 'smooth' });
};

function hideAllPages() {
    document.getElementById('home-page').classList.add('hidden');
    document.getElementById('packages-page').classList.add('hidden');
    document.getElementById('analytics-page').classList.add('hidden');
    document.getElementById('api-page').classList.add('hidden');
}

// Modal functions with enhanced animations - make them globally available
window.showLogin = function() {
    const modal = document.getElementById('login-modal');
    modal.classList.remove('hidden');
    modal.classList.add('flex');
    
    // Focus first input after animation
    setTimeout(() => {
        const firstInput = modal.querySelector('input[type="email"]');
        if (firstInput) firstInput.focus();
    }, 100);
};

window.hideLogin = function() {
    const modal = document.getElementById('login-modal');
    modal.classList.add('hidden');
    modal.classList.remove('flex');
    
    // Clear form
    const form = modal.querySelector('form');
    if (form) form.reset();
    
    // Clear response messages
    const response = document.getElementById('login-response');
    if (response) response.innerHTML = '';
};

window.showRegister = function() {
    const modal = document.getElementById('register-modal');
    modal.classList.remove('hidden');
    modal.classList.add('flex');
    
    // Focus first input after animation
    setTimeout(() => {
        const firstInput = modal.querySelector('input[type="text"]');
        if (firstInput) firstInput.focus();
    }, 100);
};

window.hideRegister = function() {
    const modal = document.getElementById('register-modal');
    modal.classList.add('hidden');
    modal.classList.remove('flex');
    
    // Clear form
    const form = modal.querySelector('form');
    if (form) form.reset();
    
    // Clear response messages
    const response = document.getElementById('register-response');
    if (response) response.innerHTML = '';
};

window.showModal = function() {
    const modal = document.getElementById('generic-modal');
    modal.classList.remove('hidden');
    modal.classList.add('flex');
};

window.hideModal = function() {
    const modal = document.getElementById('generic-modal');
    modal.classList.add('hidden');
    modal.classList.remove('flex');
    
    // Clear modal content
    const content = document.getElementById('modal-content');
    if (content) content.innerHTML = '';
};

window.logout = function() {
    authToken = null;
    currentUser = null;
    localStorage.removeItem('auth_token');
    localStorage.removeItem('user_data');
    updateAuthUI();
    showHome();
    
    // Show success message
    showNotification('Logged out successfully', 'success');
};

// Enhanced notification system
function showNotification(message, type = 'info') {
    // Remove existing notifications
    const existing = document.querySelectorAll('.notification');
    existing.forEach(n => n.remove());
    
    const notification = document.createElement('div');
    notification.className = `notification fixed top-4 right-4 z-50 px-6 py-4 rounded-lg shadow-lg transform transition-all duration-300 translate-x-full`;
    
    const colors = {
        success: 'bg-green-500 text-white',
        error: 'bg-red-500 text-white',
        info: 'bg-blue-500 text-white',
        warning: 'bg-yellow-500 text-black'
    };
    
    notification.className += ` ${colors[type] || colors.info}`;
    notification.textContent = message;
    
    document.body.appendChild(notification);
    
    // Animate in
    setTimeout(() => {
        notification.classList.remove('translate-x-full');
    }, 10);
    
    // Auto dismiss after 3 seconds
    setTimeout(() => {
        notification.classList.add('translate-x-full');
        setTimeout(() => notification.remove(), 300);
    }, 3000);
}

// HTMX response handlers
function setupHTMXHandlers() {
    // Add loading states
    document.body.addEventListener('htmx:beforeRequest', function(evt) {
        const target = evt.detail.target;
        if (target) {
            target.classList.add('opacity-75');
            
            // Add loading spinner for certain elements
            if (target.id === 'packages-list' || target.id === 'latest-packages') {
                target.innerHTML = `
                    <div class="flex justify-center items-center py-12">
                        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
                    </div>
                `;
            }
        }
    });
    
    // Remove loading states
    document.body.addEventListener('htmx:afterRequest', function(evt) {
        const target = evt.detail.target;
        if (target) {
            target.classList.remove('opacity-75');
        }
        
        // Handle packages list response
        if (evt.detail.xhr.responseURL && evt.detail.xhr.responseURL.includes('/api/packages')) {
            try {
                const data = JSON.parse(evt.detail.xhr.responseText);
                if (data.packages) {
                    renderPackages(data.packages, evt.detail.target);
                }
            } catch (error) {
                console.error('Error parsing packages response:', error);
                evt.detail.target.innerHTML = `
                    <div class="text-center py-12">
                        <div class="text-red-500 mb-4">
                            <svg class="w-16 h-16 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                            </svg>
                            <p class="text-lg font-medium">Error loading packages</p>
                            <p class="text-sm text-gray-500 mt-2">Please try refreshing the page</p>
                        </div>
                    </div>
                `;
            }
        }
        
        // Handle auth responses
        if (evt.detail.xhr.responseURL && evt.detail.xhr.responseURL.includes('/api/auth/')) {
            if (evt.detail.xhr.status === 200) {
                try {
                    const data = JSON.parse(evt.detail.xhr.responseText);
                    if (data.token) {
                        authToken = data.token;
                        currentUser = { username: data.user_id || 'User' };
                        
                        localStorage.setItem('auth_token', authToken);
                        localStorage.setItem('user_data', JSON.stringify(currentUser));
                        
                        updateAuthUI();
                        hideLogin();
                        hideRegister();
                        showHome();
                        
                        showNotification('Successfully logged in!', 'success');
                    }
                } catch (error) {
                    showNotification('Authentication error', 'error');
                }
            } else {
                showNotification('Authentication failed. Please check your credentials.', 'error');
            }
        }
    });
    
    // Handle HTMX errors
    document.body.addEventListener('htmx:responseError', function(evt) {
        console.error('HTMX Error:', evt.detail);
        showNotification('Network error. Please try again.', 'error');
    });
}

function renderPackages(packages, target) {
    if (packages.length === 0) {
        target.innerHTML = `
            <div class="text-center py-12">
                <div class="text-gray-400 mb-4">
                    <svg class="w-16 h-16 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-4.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 009.586 13H7"></path>
                    </svg>
                    <p class="text-lg font-medium text-gray-600">No packages found</p>
                    <p class="text-sm text-gray-500 mt-2">Try adjusting your search criteria or add a new package</p>
                </div>
            </div>
        `;
        return;
    }
    
    // Update results count
    updateResultsCount(packages.length);
    
    if (currentViewMode === 'list') {
        renderPackagesList(packages, target);
    } else {
        renderPackagesGrid(packages, target);
    }
}

function renderPackagesGrid(packages, target) {
    const html = packages.map(pkg => `
        <div class="bg-gray-800 rounded-2xl shadow-lg p-6 card-hover border border-gray-700 relative" data-package-id="${pkg.id}">
            <!-- Package Actions -->
            <div class="absolute top-4 right-4 flex space-x-2">
                <button onclick="toggleFavorite('${pkg.id}')" class="p-2 rounded-lg hover:bg-gray-100 transition-colors favorite-btn" data-package-id="${pkg.id}">
                    <svg class="w-4 h-4 ${favoritesCache.has(pkg.id) ? 'text-red-500 fill-current' : 'text-gray-400'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z"></path>
                    </svg>
                </button>
                <button onclick="addToComparison('${pkg.id}')" class="p-2 rounded-lg hover:bg-gray-100 transition-colors" title="Add to comparison">
                    <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"></path>
                    </svg>
                </button>
            </div>
            
            <div class="flex justify-between items-start mb-4">
                <div class="flex-1 pr-12">
                    <h3 class="text-xl font-bold text-gray-100 mb-2 hover:text-blue-400 transition-colors cursor-pointer" onclick="showPackageDetails('${pkg.id}')">${pkg.name}</h3>
                    <p class="text-gray-600 leading-relaxed">${pkg.description || 'No description available'}</p>
                </div>
                <div class="text-right">
                    <div class="text-xs text-gray-500 bg-gray-100 px-2 py-1 rounded-full">
                        ${new Date(pkg.created_at).toLocaleDateString()}
                    </div>
                </div>
            </div>
            
            ${pkg.tags && pkg.tags.length > 0 ? `
                <div class="flex flex-wrap gap-2 mb-4">
                    ${pkg.tags.map(tag => `
                        <span class="bg-gradient-to-r from-blue-100 to-purple-100 text-blue-800 px-3 py-1 rounded-full text-xs font-medium border border-blue-200 cursor-pointer hover:from-blue-200 hover:to-purple-200 transition-colors" onclick="filterByTag('${tag}')">
                            ${tag}
                        </span>
                    `).join('')}
                </div>
            ` : ''}
            
            <div class="flex justify-between items-center pt-4 border-t border-gray-100">
                <div class="text-sm text-gray-600">
                    ${pkg.license ? `
                        <span class="bg-green-100 text-green-800 px-2 py-1 rounded text-xs font-medium">
                            ${pkg.license}
                        </span>
                    ` : '<span class="text-gray-400">No license</span>'}
                </div>
                <div class="flex space-x-3">
                    ${pkg.homepage ? `
                        <a href="${pkg.homepage}" target="_blank" class="text-blue-600 hover:text-blue-800 transition-colors" title="Homepage">
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"></path>
                            </svg>
                        </a>
                    ` : ''}
                    ${pkg.repository ? `
                        <a href="${pkg.repository}" target="_blank" class="text-gray-600 hover:text-gray-800 transition-colors" title="Repository">
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"></path>
                            </svg>
                        </a>
                    ` : ''}
                </div>
            </div>
        </div>
    `).join('');
    
    target.innerHTML = html;
    target.className = 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6';
}

function renderPackagesList(packages, target) {
    const html = packages.map(pkg => `
        <div class="bg-gray-800 rounded-xl shadow-lg p-6 card-hover border border-gray-700 flex items-center space-x-6" data-package-id="${pkg.id}">
            <div class="flex-1">
                <div class="flex items-start justify-between mb-2">
                    <h3 class="text-xl font-bold text-gray-800 hover:text-blue-600 transition-colors cursor-pointer" onclick="showPackageDetails('${pkg.id}')">${pkg.name}</h3>
                    <div class="flex space-x-2">
                        <button onclick="toggleFavorite('${pkg.id}')" class="p-1 rounded hover:bg-gray-100 transition-colors favorite-btn" data-package-id="${pkg.id}">
                            <svg class="w-4 h-4 ${favoritesCache.has(pkg.id) ? 'text-red-500 fill-current' : 'text-gray-400'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z"></path>
                            </svg>
                        </button>
                        <button onclick="addToComparison('${pkg.id}')" class="p-1 rounded hover:bg-gray-100 transition-colors" title="Add to comparison">
                            <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"></path>
                            </svg>
                        </button>
                    </div>
                </div>
                <p class="text-gray-600 mb-3">${pkg.description || 'No description available'}</p>
                
                ${pkg.tags && pkg.tags.length > 0 ? `
                    <div class="flex flex-wrap gap-2 mb-3">
                        ${pkg.tags.slice(0, 3).map(tag => `
                            <span class="bg-gradient-to-r from-blue-100 to-purple-100 text-blue-800 px-2 py-1 rounded-full text-xs font-medium border border-blue-200 cursor-pointer hover:from-blue-200 hover:to-purple-200 transition-colors" onclick="filterByTag('${tag}')">
                                ${tag}
                            </span>
                        `).join('')}
                        ${pkg.tags.length > 3 ? `<span class="text-xs text-gray-500">+${pkg.tags.length - 3} more</span>` : ''}
                    </div>
                ` : ''}
                
                <div class="flex items-center justify-between">
                    <div class="flex items-center space-x-4">
                        ${pkg.license ? `
                            <span class="bg-green-100 text-green-800 px-2 py-1 rounded text-xs font-medium">
                                ${pkg.license}
                            </span>
                        ` : '<span class="text-gray-400 text-xs">No license</span>'}
                        <span class="text-xs text-gray-500">
                            ${new Date(pkg.created_at).toLocaleDateString()}
                        </span>
                    </div>
                    <div class="flex space-x-3">
                        ${pkg.homepage ? `
                            <a href="${pkg.homepage}" target="_blank" class="text-blue-600 hover:text-blue-800 transition-colors" title="Homepage">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"></path>
                                </svg>
                            </a>
                        ` : ''}
                        ${pkg.repository ? `
                            <a href="${pkg.repository}" target="_blank" class="text-gray-600 hover:text-gray-800 transition-colors" title="Repository">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"></path>
                                </svg>
                            </a>
                        ` : ''}
                    </div>
                </div>
            </div>
        </div>
    `).join('');
    
    target.innerHTML = html;
    target.className = 'space-y-4';
}

// Enhanced keyboard shortcuts
document.addEventListener('keydown', function(evt) {
    // Close modals with Escape key
    if (evt.key === 'Escape') {
        hideLogin();
        hideRegister();
        hideModal();
    }
    
    // Quick navigation with Ctrl/Cmd + key
    if (evt.ctrlKey || evt.metaKey) {
        switch(evt.key) {
            case '1':
                evt.preventDefault();
                showHome();
                break;
            case '2':
                evt.preventDefault();
                showPackages();
                break;
            case '3':
                evt.preventDefault();
                showAnalytics();
                break;
            case '4':
                evt.preventDefault();
                showAPI();
                break;
            case 'k':
                evt.preventDefault();
                const searchInput = document.querySelector('input[name="search"]');
                if (searchInput) searchInput.focus();
                break;
        }
    }
});

// Enhanced scroll effects
let lastScrollY = window.scrollY;
const nav = document.querySelector('nav');

window.addEventListener('scroll', () => {
    if (window.scrollY > lastScrollY && window.scrollY > 100) {
        // Scrolling down
        nav.style.transform = 'translateY(-100%)';
    } else {
        // Scrolling up
        nav.style.transform = 'translateY(0)';
    }
    lastScrollY = window.scrollY;
});

// Add smooth scrolling for anchor links
document.addEventListener('click', function(e) {
    if (e.target.tagName === 'A' && e.target.getAttribute('href')?.startsWith('#')) {
        e.preventDefault();
        const target = document.querySelector(e.target.getAttribute('href'));
        if (target) {
            target.scrollIntoView({ behavior: 'smooth' });
        }
    }
});

// Add intersection observer for animations
const observerOptions = {
    threshold: 0.1,
    rootMargin: '0px 0px -50px 0px'
};

const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
        if (entry.isIntersecting) {
            entry.target.classList.add('animate-fadeIn');
        }
    });
}, observerOptions);

// Observe elements for animation
document.addEventListener('DOMContentLoaded', () => {
    const animateElements = document.querySelectorAll('.card-hover, .glass-effect');
    animateElements.forEach(el => observer.observe(el));
});

// View mode and utility functions
function updateViewMode(mode) {
    currentViewMode = mode;
    localStorage.setItem('viewMode', mode);
    
    // Refresh packages list if visible
    const packagesList = document.getElementById('packages-list');
    if (packagesList && !packagesList.classList.contains('hidden')) {
        const trigger = document.querySelector('[name="search"]');
        if (trigger) {
            htmx.trigger(trigger, 'keyup');
        }
    }
}

function clearAllFilters() {
    document.querySelectorAll('.search-filter').forEach(input => {
        if (input.tagName === 'SELECT') {
            input.selectedIndex = 0;
        } else {
            input.value = '';
        }
    });
    
    // Trigger search refresh
    const searchInput = document.querySelector('[name="search"]');
    if (searchInput) {
        htmx.trigger(searchInput, 'keyup');
    }
}

function updateResultsCount(count) {
    const resultsCount = document.getElementById('results-count');
    if (resultsCount) {
        resultsCount.textContent = `Showing ${count} package${count !== 1 ? 's' : ''}`;
    }
}

function filterByTag(tag) {
    const tagSelect = document.querySelector('[name="category"]');
    if (tagSelect) {
        tagSelect.value = tag;
        htmx.trigger(tagSelect, 'change');
    }
}

// Favorites functionality
function toggleFavorite(packageId) {
    if (favoritesCache.has(packageId)) {
        favoritesCache.delete(packageId);
        showNotification('Removed from favorites', 'info');
    } else {
        favoritesCache.add(packageId);
        showNotification('Added to favorites', 'success');
    }
    
    // Update UI
    updateFavoriteButtons(packageId);
    
    // Save to localStorage
    localStorage.setItem('favorites', JSON.stringify([...favoritesCache]));
}

function updateFavoriteButtons(packageId) {
    const buttons = document.querySelectorAll(`[data-package-id="${packageId}"]`);
    buttons.forEach(button => {
        const svg = button.querySelector('svg');
        if (svg) {
            if (favoritesCache.has(packageId)) {
                svg.classList.add('text-red-500', 'fill-current');
                svg.classList.remove('text-gray-400');
            } else {
                svg.classList.remove('text-red-500', 'fill-current');
                svg.classList.add('text-gray-400');
            }
        }
    });
}

// Package comparison functionality
function addToComparison(packageId) {
    if (comparisonList.includes(packageId)) {
        showNotification('Package already in comparison', 'warning');
        return;
    }
    
    if (comparisonList.length >= 3) {
        showNotification('Maximum 3 packages can be compared', 'warning');
        return;
    }
    
    comparisonList.push(packageId);
    showNotification(`Added to comparison (${comparisonList.length}/3)`, 'success');
    updateComparisonUI();
}

function removeFromComparison(packageId) {
    comparisonList = comparisonList.filter(id => id !== packageId);
    updateComparisonUI();
}

function updateComparisonUI() {
    // Create or update comparison bar if packages are selected
    let comparisonBar = document.getElementById('comparison-bar');
    
    if (comparisonList.length === 0) {
        if (comparisonBar) comparisonBar.remove();
        return;
    }
    
    if (!comparisonBar) {
        comparisonBar = document.createElement('div');
        comparisonBar.id = 'comparison-bar';
        comparisonBar.className = 'fixed bottom-4 right-4 bg-gray-800 rounded-lg shadow-xl p-4 border border-gray-700 z-50 max-w-sm';
        document.body.appendChild(comparisonBar);
    }
    
    comparisonBar.innerHTML = `
        <div class="flex items-center justify-between mb-3">
            <h4 class="font-semibold text-gray-100">Compare Packages</h4>
            <button onclick="clearComparison()" class="text-gray-400 hover:text-gray-600">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                </svg>
            </button>
        </div>
        <div class="space-y-2 mb-3">
            ${comparisonList.map(id => `
                <div class="flex items-center justify-between text-sm">
                    <span class="text-gray-600">Package ${id.slice(0, 8)}...</span>
                    <button onclick="removeFromComparison('${id}')" class="text-red-500 hover:text-red-700">
                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </div>
            `).join('')}
        </div>
        <button onclick="showComparison()" class="w-full px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 transition-colors">
            Compare (${comparisonList.length})
        </button>
    `;
}

function clearComparison() {
    comparisonList = [];
    updateComparisonUI();
}

function showComparison() {
    // This would open a comparison modal/page
    showNotification('Comparison feature coming soon!', 'info');
}

function showPackageDetails(packageId) {
    // This would show detailed package information
    showNotification('Package details feature coming soon!', 'info');
}

// Analytics functionality
function loadAnalyticsData() {
    // Simulate loading real analytics data
    animateCounters();
    updateCharts();
}

function animateCounters() {
    const counters = [
        { id: 'total-packages', target: 1234567, duration: 2000 },
        { id: 'active-maintainers', target: 89123, duration: 1800 },
        { id: 'programming-languages', target: 156, duration: 1000 },
        { id: 'weekly-updates', target: 45678, duration: 1500 }
    ];
    
    counters.forEach(counter => {
        const element = document.getElementById(counter.id);
        if (element) {
            let start = 0;
            const startTime = Date.now();
            
            const animate = () => {
                const elapsed = Date.now() - startTime;
                const progress = Math.min(elapsed / counter.duration, 1);
                
                // Easing function
                const easeOut = 1 - Math.pow(1 - progress, 3);
                const current = Math.floor(start + (counter.target - start) * easeOut);
                
                element.textContent = current.toLocaleString();
                
                if (progress < 1) {
                    requestAnimationFrame(animate);
                }
            };
            
            animate();
        }
    });
}

function updateCharts() {
    // Add sparkline animations
    const chartBars = document.querySelectorAll('#analytics-page .bg-gradient-to-t');
    chartBars.forEach((bar, index) => {
        setTimeout(() => {
            bar.style.transition = 'height 0.8s ease-out';
            bar.style.height = bar.style.height || '50%';
        }, index * 100);
    });
}

// Load saved preferences
document.addEventListener('DOMContentLoaded', () => {
    // Load view mode preference
    const savedViewMode = localStorage.getItem('viewMode');
    if (savedViewMode) {
        currentViewMode = savedViewMode;
    }
    
    // Load favorites
    const savedFavorites = localStorage.getItem('favorites');
    if (savedFavorites) {
        favoritesCache = new Set(JSON.parse(savedFavorites));
    }
});

// Add fade-in animation class via CSS
const style = document.createElement('style');
style.textContent = `
    .animate-fadeIn {
        animation: fadeIn 0.6s ease-out forwards;
    }
    
    @keyframes fadeIn {
        from {
            opacity: 0;
            transform: translateY(20px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }
    
    nav {
        transition: transform 0.3s ease-in-out;
    }
    
    #comparison-bar {
        animation: slideInRight 0.3s ease-out;
    }
    
    @keyframes slideInRight {
        from {
            transform: translateX(100%);
            opacity: 0;
        }
        to {
            transform: translateX(0);
            opacity: 1;
        }
    }
`;
document.head.appendChild(style);