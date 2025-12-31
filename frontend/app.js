// Global state
let currentUser = null;
let authToken = null;
let currentViewMode = 'grid';
let subscriptionsCache = new Set();
let comparisonList = [];
let timelineWebSocket = null;
let timelineOffset = 0;
let timelineLimit = 20;
let timelineTotal = 0;
let timelineLoading = false;
let displayedEventIds = new Set();
let timelineUpdateInterval = null;

// Initialize app
document.addEventListener('DOMContentLoaded', function() {
    // Hide loading screen
    const loadingScreen = document.getElementById('loading-screen');
    if (loadingScreen) {
        loadingScreen.style.display = 'none';
    }

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
    
    // Clear auth errors when user starts typing
    setupAuthErrorClearing();
});

function checkAuthStatus() {
    const token = localStorage.getItem('auth_token');
    const userData = localStorage.getItem('user_data');

    if (token && userData) {
        authToken = token;
        currentUser = JSON.parse(userData);
        updateAuthUI();
        loadSubscriptionsCache(); // Load user's subscriptions
    }
}

function updateAuthUI() {
    const authButtons = document.getElementById('auth-buttons');
    const userMenu = document.getElementById('user-menu');
    const subscriptionsNav = document.getElementById('nav-subscriptions');
    const mobileSubscriptionsNav = document.getElementById('mobile-nav-subscriptions');

    if (currentUser) {
        authButtons.classList.add('hidden');
        userMenu.classList.remove('hidden');
        document.getElementById('username').textContent = currentUser.username;

        // Show subscriptions link when logged in
        if (subscriptionsNav) subscriptionsNav.classList.remove('hidden');
        if (mobileSubscriptionsNav) mobileSubscriptionsNav.classList.remove('hidden');
    } else {
        authButtons.classList.remove('hidden');
        userMenu.classList.add('hidden');

        // Hide subscriptions link when logged out
        if (subscriptionsNav) subscriptionsNav.classList.add('hidden');
        if (mobileSubscriptionsNav) mobileSubscriptionsNav.classList.add('hidden');
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
    hideAllPages();
    const homePage = document.getElementById('home-page');
    if (homePage) {
        homePage.classList.remove('hidden');
    }

    // Show timeline for logged-in users, latest packages for logged-out users
    const timelineSection = document.getElementById('timeline-section');
    const latestPackagesSection = document.getElementById('latest-packages-section');

    if (currentUser && authToken) {
        // User is logged in - show personal timeline
        if (timelineSection) {
            timelineSection.classList.remove('hidden');
        }
        if (latestPackagesSection) {
            latestPackagesSection.classList.add('hidden');
        }
        loadTimeline();
    } else {
        // User is logged out - show global timeline
        if (timelineSection) {
            timelineSection.classList.remove('hidden');
        }
        if (latestPackagesSection) {
            latestPackagesSection.classList.add('hidden');
        }
        loadTimeline(); // Load global timeline for logged-out users
    }

    updateActiveNav('home');
    window.scrollTo({ top: 0, behavior: 'smooth' });
};

window.showPackages = function() {
    disconnectTimelineWebSocket(); // Disconnect websocket when leaving timeline
    stopTimelineUpdates(); // Stop timestamp updates when leaving timeline
    hideAllPages();
    const packagesPage = document.getElementById('packages-page');
    if (packagesPage) {
        packagesPage.classList.remove('hidden');
    }
    updateActiveNav('packages');
    window.scrollTo({ top: 0, behavior: 'smooth' });
};

// Analytics page removed - use /api/analytics endpoint for data

window.showSubscriptions = function() {
    if (!currentUser || !authToken) {
        showNotification('Please log in to view subscriptions', 'warning');
        showLogin();
        return;
    }

    disconnectTimelineWebSocket(); // Disconnect websocket when leaving timeline
    stopTimelineUpdates(); // Stop timestamp updates when leaving timeline
    hideAllPages();
    const subscriptionsPage = document.getElementById('subscriptions-page');
    if (subscriptionsPage) {
        subscriptionsPage.classList.remove('hidden');
    }
    updateActiveNav('subscriptions');
    loadSubscriptions();
    window.scrollTo({ top: 0, behavior: 'smooth' });
};

window.showAPI = function() {
    disconnectTimelineWebSocket(); // Disconnect websocket when leaving timeline
    stopTimelineUpdates(); // Stop timestamp updates when leaving timeline
    hideAllPages();
    const apiPage = document.getElementById('api-page');
    if (apiPage) {
        apiPage.classList.remove('hidden');
    }
    updateActiveNav('api');
    window.scrollTo({ top: 0, behavior: 'smooth' });
};

function hideAllPages() {
    const homePage = document.getElementById('home-page');
    const packagesPage = document.getElementById('packages-page');
    const subscriptionsPage = document.getElementById('subscriptions-page');
    const apiPage = document.getElementById('api-page');

    if (homePage) homePage.classList.add('hidden');
    if (packagesPage) packagesPage.classList.add('hidden');
    if (subscriptionsPage) subscriptionsPage.classList.add('hidden');
    if (apiPage) apiPage.classList.add('hidden');
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
    
    // Remove any shake animation
    const formContainer = modal.querySelector('.bg-gray-800');
    if (formContainer) {
        formContainer.classList.remove('animate-shake');
    }
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
    
    // Remove any shake animation
    const formContainer = modal.querySelector('.bg-gray-800');
    if (formContainer) {
        formContainer.classList.remove('animate-shake');
    }
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
    subscriptionsCache.clear();
    localStorage.removeItem('auth_token');
    localStorage.removeItem('user_data');
    localStorage.removeItem('subscriptions');
    updateAuthUI();
    showHome();

    // Show success message
    showNotification('Logged out successfully', 'success');
};

// Email validation function
function isValidEmail(email) {
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return emailRegex.test(email);
}

// Show email validation error
function showEmailError(type, email) {
    const message = email.length === 0 ? 
        'Email address is required.' :
        'Please enter a valid email address (e.g., user@example.com).';
    
    // Add visual indication to the email field
    const emailInput = document.querySelector(`#${type}-modal input[type="email"]`);
    if (emailInput) {
        emailInput.classList.add('border-red-500', 'ring-red-400');
        emailInput.classList.remove('border-gray-600', 'ring-blue-400');
    }
    
    showAuthError(type, message);
}

// Clear email field error styling
function clearEmailError(type) {
    const emailInput = document.querySelector(`#${type}-modal input[type="email"]`);
    if (emailInput) {
        emailInput.classList.remove('border-red-500', 'ring-red-400');
        emailInput.classList.add('border-gray-600');
    }
}

// Setup error clearing and validation on input
function setupAuthErrorClearing() {
    // Setup login form validation
    const loginEmailInput = document.querySelector('#login-modal input[type="email"]');
    const loginPasswordInput = document.querySelector('#login-modal input[type="password"]');
    const loginForm = document.querySelector('#login-modal form');
    
    if (loginEmailInput) {
        loginEmailInput.addEventListener('input', function() {
            const response = document.getElementById('login-response');
            if (response && response.innerHTML.includes('bg-red-500')) {
                response.innerHTML = '';
            }
            clearEmailError('login');
        });
        
        loginEmailInput.addEventListener('blur', function() {
            const email = this.value.trim();
            if (email.length > 0 && !isValidEmail(email)) {
                showEmailError('login', email);
            }
        });
    }
    
    if (loginPasswordInput) {
        loginPasswordInput.addEventListener('input', function() {
            const response = document.getElementById('login-response');
            if (response && response.innerHTML.includes('bg-red-500')) {
                response.innerHTML = '';
            }
        });
    }
    
    if (loginForm) {
        loginForm.addEventListener('submit', function(e) {
            const email = loginEmailInput.value.trim();
            if (!isValidEmail(email)) {
                e.preventDefault();
                e.stopPropagation();
                showEmailError('login', email);
                return false;
            }
        });
    }
    
    // Setup register form validation
    const registerEmailInput = document.querySelector('#register-modal input[type="email"]');
    const registerUsernameInput = document.querySelector('#register-modal input[type="text"]');
    const registerPasswordInput = document.querySelector('#register-modal input[type="password"]');
    const registerForm = document.querySelector('#register-modal form');
    
    if (registerEmailInput) {
        registerEmailInput.addEventListener('input', function() {
            const response = document.getElementById('register-response');
            if (response && response.innerHTML.includes('bg-red-500')) {
                response.innerHTML = '';
            }
            clearEmailError('register');
        });
        
        registerEmailInput.addEventListener('blur', function() {
            const email = this.value.trim();
            if (email.length > 0 && !isValidEmail(email)) {
                showEmailError('register', email);
            }
        });
    }
    
    if (registerUsernameInput) {
        registerUsernameInput.addEventListener('input', function() {
            const response = document.getElementById('register-response');
            if (response && response.innerHTML.includes('bg-red-500')) {
                response.innerHTML = '';
            }
        });
    }
    
    if (registerPasswordInput) {
        registerPasswordInput.addEventListener('input', function() {
            const response = document.getElementById('register-response');
            if (response && response.innerHTML.includes('bg-red-500')) {
                response.innerHTML = '';
            }
        });
    }
    
    if (registerForm) {
        registerForm.addEventListener('submit', function(e) {
            const email = registerEmailInput.value.trim();
            const username = registerUsernameInput.value.trim();
            const password = registerPasswordInput.value.trim();
            
            // Validate email
            if (!isValidEmail(email)) {
                e.preventDefault();
                e.stopPropagation();
                showEmailError('register', email);
                return false;
            }
            
            // Validate username
            if (username.length < 3) {
                e.preventDefault();
                e.stopPropagation();
                showAuthError('register', 'Username must be at least 3 characters long.');
                return false;
            }
            
            // Validate password
            if (password.length < 6) {
                e.preventDefault();
                e.stopPropagation();
                showAuthError('register', 'Password must be at least 6 characters long.');
                return false;
            }
        });
    }
}

// Auth error display function
function showAuthError(type, message) {
    const responseId = type === 'login' ? 'login-response' : 'register-response';
    const responseElement = document.getElementById(responseId);
    
    if (responseElement) {
        responseElement.innerHTML = `
            <div class="p-3 mb-4 bg-red-500/10 border border-red-500/20 rounded-lg">
                <div class="flex items-center space-x-3">
                    <svg class="w-5 h-5 text-red-400 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                    </svg>
                    <div class="text-sm text-red-300">${message}</div>
                </div>
            </div>
        `;
        
        // Add shake animation to the form
        const modal = document.getElementById(`${type}-modal`);
        if (modal) {
            const formContainer = modal.querySelector('.bg-gray-800');
            if (formContainer) {
                formContainer.classList.add('animate-shake');
                setTimeout(() => {
                    formContainer.classList.remove('animate-shake');
                }, 600);
            }
        }
    }
}

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
            
            // Show loading state in auth forms
            if (target.id === 'login-response' || target.id === 'register-response') {
                target.innerHTML = `
                    <div class="flex items-center justify-center p-3 mb-4 bg-blue-500/10 border border-blue-500/20 rounded-lg">
                        <div class="flex items-center space-x-3">
                            <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-400"></div>
                            <div class="text-sm text-blue-300">Authenticating...</div>
                        </div>
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

        // Handle 401 Unauthorized - token expired or invalid
        if (evt.detail.xhr.status === 401) {
            // Clear auth state
            authToken = null;
            currentUser = null;
            localStorage.removeItem('auth_token');
            localStorage.removeItem('user_data');
            updateAuthUI();

            showNotification('Session expired. Please log in again.', 'warning');

            // Only show login modal if this wasn't already an auth request
            if (!evt.detail.xhr.responseURL.includes('/api/auth/')) {
                setTimeout(() => showLogin(), 500);
            }
            return;
        }

        // Handle packages list response
        if (evt.detail.xhr.responseURL && evt.detail.xhr.responseURL.includes('/api/packages')) {
            try {
                const data = JSON.parse(evt.detail.xhr.responseText);
                if (data.packages) {
                    renderPackages(data.packages, evt.detail.target);
                }
            } catch (error) {
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
        
        // Handle stats response
        if (evt.detail.xhr.responseURL && evt.detail.xhr.responseURL.includes('/api/stats')) {
            if (evt.detail.xhr.status === 200) {
                try {
                    const stats = JSON.parse(evt.detail.xhr.responseText);
                    evt.detail.target.innerHTML = renderDatabaseStats(stats);
                } catch (error) {
                    evt.detail.target.innerHTML = `
                        <div class="col-span-full text-gray-400 text-sm">Failed to load stats</div>
                    `;
                }
            }
        }

        // Handle auth responses
        if (evt.detail.xhr.responseURL && evt.detail.xhr.responseURL.includes('/api/auth/')) {
            const isLogin = evt.detail.xhr.responseURL.includes('/login');
            const isRegister = evt.detail.xhr.responseURL.includes('/register');
            
            if (evt.detail.xhr.status === 200) {
                try {
                    const data = JSON.parse(evt.detail.xhr.responseText);
                    if (data.token) {
                        // Show success state briefly before closing
                        const responseId = isLogin ? 'login-response' : 'register-response';
                        const responseElement = document.getElementById(responseId);
                        
                        if (responseElement) {
                            responseElement.innerHTML = `
                                <div class="p-3 mb-4 bg-green-500/10 border border-green-500/20 rounded-lg">
                                    <div class="flex items-center space-x-3">
                                        <svg class="w-5 h-5 text-green-400 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                                        </svg>
                                        <div class="text-sm text-green-300">Success! Redirecting...</div>
                                    </div>
                                </div>
                            `;
                        }
                        
                        // Delay the redirect slightly to show success message
                        setTimeout(() => {
                            authToken = data.token;
                            currentUser = { username: data.user_id || 'User' };

                            localStorage.setItem('auth_token', authToken);
                            localStorage.setItem('user_data', JSON.stringify(currentUser));

                            updateAuthUI();
                            loadSubscriptionsCache(); // Load user's subscriptions
                            hideLogin();
                            hideRegister();
                            showHome();

                            const action = isRegister ? 'registered' : 'logged in';
                            showNotification(`Successfully ${action}!`, 'success');
                        }, 800);
                    }
                } catch (error) {
                    const errorMsg = 'Authentication response error. Please try again.';
                    showNotification(errorMsg, 'error');
                    showAuthError(isLogin ? 'login' : 'register', errorMsg);
                }
            } else {
                // Handle different error status codes
                let errorMessage = 'Authentication failed. Please try again.';
                
                if (evt.detail.xhr.status === 401) {
                    errorMessage = isLogin ? 
                        'Invalid email or password. Please check your credentials and try again.' :
                        'Account creation failed. Email may already be in use.';
                } else if (evt.detail.xhr.status === 400) {
                    errorMessage = 'Invalid request. Please check your input and try again.';
                } else if (evt.detail.xhr.status === 429) {
                    errorMessage = 'Too many attempts. Please wait before trying again.';
                } else if (evt.detail.xhr.status >= 500) {
                    errorMessage = 'Server error. Please try again later.';
                }
                
                showNotification(errorMessage, 'error');
                showAuthError(isLogin ? 'login' : 'register', errorMessage);
            }
        }
    });
    
    // Handle HTMX errors
    document.body.addEventListener('htmx:responseError', function(evt) {
        showNotification('Network error. Please try again.', 'error');
    });
}

function renderDatabaseStats(stats) {
    return `
        <div class="bg-gray-800/50 rounded-lg p-3 border border-gray-700">
            <div class="text-2xl font-bold text-blue-400 mb-1">${stats.total_packages}</div>
            <div class="text-xs text-gray-400">Packages</div>
        </div>
        <div class="bg-gray-800/50 rounded-lg p-3 border border-gray-700">
            <div class="text-2xl font-bold text-green-400 mb-1">${stats.total_versions}</div>
            <div class="text-xs text-gray-400">Versions</div>
        </div>
        <div class="bg-gray-800/50 rounded-lg p-3 border border-gray-700">
            <div class="text-2xl font-bold text-purple-400 mb-1">${stats.total_users}</div>
            <div class="text-xs text-gray-400">Users</div>
        </div>
        <div class="bg-gray-800/50 rounded-lg p-3 border border-gray-700">
            <div class="text-2xl font-bold text-yellow-400 mb-1">${stats.total_vulnerabilities}</div>
            <div class="text-xs text-gray-400">CVEs</div>
        </div>
    `;
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
            <!-- Comparison button in top-right -->
            <div class="absolute top-4 right-4">
                <button onclick="addToComparison('${pkg.id}')" class="p-2 rounded-lg hover:bg-gray-700 transition-colors" title="Add to comparison">
                    <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"></path>
                    </svg>
                </button>
            </div>

            <div class="mb-4">
                <div class="pr-12">
                    <h3 class="text-xl font-bold text-gray-100 mb-2 hover:text-blue-400 transition-colors cursor-pointer" onclick="showPackageDetails('${pkg.id}')">${pkg.name}</h3>
                    <p class="text-gray-400 leading-relaxed">${pkg.description || 'No description available'}</p>
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
            
            <div class="flex justify-between items-center pt-4 border-t border-gray-700">
                <div class="flex items-center space-x-3">
                    ${pkg.license ? `
                        <span class="bg-green-500/10 text-green-400 px-2 py-1 rounded text-xs font-medium border border-green-500/20">
                            ${pkg.license}
                        </span>
                    ` : '<span class="text-gray-400 text-xs">No license</span>'}
                    <span class="text-xs text-gray-500">
                        ${new Date(pkg.created_at).toLocaleDateString()}
                    </span>
                </div>
                <div class="flex space-x-3">
                    ${pkg.homepage ? `
                        <a href="${pkg.homepage}" target="_blank" class="text-blue-400 hover:text-blue-300 transition-colors" title="Homepage">
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"></path>
                            </svg>
                        </a>
                    ` : ''}
                    ${pkg.repository ? `
                        <a href="${pkg.repository}" target="_blank" class="text-gray-400 hover:text-gray-300 transition-colors" title="Repository">
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
        <div class="bg-gray-800 rounded-xl shadow-lg p-6 card-hover border border-gray-700 relative" data-package-id="${pkg.id}">
            <!-- Comparison button in top-right -->
            <div class="absolute top-4 right-4">
                <button onclick="addToComparison('${pkg.id}')" class="p-2 rounded-lg hover:bg-gray-700 transition-colors" title="Add to comparison">
                    <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"></path>
                    </svg>
                </button>
            </div>

            <div class="flex-1 pr-12">
                <div class="mb-2">
                    <h3 class="text-xl font-bold text-gray-100 hover:text-blue-400 transition-colors cursor-pointer" onclick="showPackageDetails('${pkg.id}')">${pkg.name}</h3>
                </div>
                <p class="text-gray-400 mb-3">${pkg.description || 'No description available'}</p>
                
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

// Subscription toggle functionality
async function toggleSubscription(packageName) {
    if (!currentUser || !authToken) {
        showNotification('Please log in to subscribe to packages', 'warning');
        showLogin();
        return;
    }

    if (subscriptionsCache.has(packageName)) {
        // Unsubscribe
        try {
            const response = await fetch(`/api/users/subscriptions/${encodeURIComponent(packageName)}`, {
                method: 'DELETE',
                headers: {
                    'Authorization': `Bearer ${authToken}`
                }
            });

            if (!response.ok) {
                throw new Error('Failed to unsubscribe');
            }

            subscriptionsCache.delete(packageName);
            showNotification(`Unsubscribed from ${packageName}`, 'info');
            updateSubscriptionButtons(packageName);
            saveSubscriptionsToCache();
        } catch (error) {
            showNotification('Failed to unsubscribe. Please try again.', 'error');
        }
    } else {
        // Subscribe
        try {
            const response = await fetch('/api/users/subscriptions', {
                method: 'POST',
                headers: {
                    'Authorization': `Bearer ${authToken}`,
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({ package_name: packageName })
            });

            if (!response.ok) {
                throw new Error('Failed to subscribe');
            }

            subscriptionsCache.add(packageName);
            showNotification(`Subscribed to ${packageName}`, 'success');
            updateSubscriptionButtons(packageName);
            saveSubscriptionsToCache();
        } catch (error) {
            showNotification('Failed to subscribe. Please try again.', 'error');
        }
    }
}

function updateSubscriptionButtons(packageName) {
    const buttons = document.querySelectorAll(`[data-package-name="${packageName}"]`);
    buttons.forEach(button => {
        const svg = button.querySelector('svg');
        if (svg) {
            if (subscriptionsCache.has(packageName)) {
                svg.classList.add('text-red-500', 'fill-current');
                svg.classList.remove('text-gray-400');
                button.setAttribute('title', 'Unsubscribe');
            } else {
                svg.classList.remove('text-red-500', 'fill-current');
                svg.classList.add('text-gray-400');
                button.setAttribute('title', 'Subscribe');
            }
        }
    });
}

function saveSubscriptionsToCache() {
    localStorage.setItem('subscriptions', JSON.stringify([...subscriptionsCache]));
}

async function loadSubscriptionsCache() {
    if (!currentUser || !authToken) {
        subscriptionsCache.clear();
        return;
    }

    try {
        const response = await fetch('/api/users/subscriptions', {
            headers: {
                'Authorization': `Bearer ${authToken}`
            }
        });

        if (response.ok) {
            const data = await response.json();
            subscriptionsCache = new Set(data.subscriptions || []);
            saveSubscriptionsToCache();
        }
    } catch (error) {
        // Fallback to localStorage
        const savedSubscriptions = localStorage.getItem('subscriptions');
        if (savedSubscriptions) {
            subscriptionsCache = new Set(JSON.parse(savedSubscriptions));
        }
    }
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

async function showComparison() {
    if (comparisonList.length < 2) {
        showNotification('Add at least 2 packages to compare', 'warning');
        return;
    }

    try {
        const packages = await Promise.all(
            comparisonList.map(id =>
                fetch(`/api/packages/${id}`).then(r => r.json())
            )
        );

        const modalContent = document.getElementById('modal-content');
        modalContent.innerHTML = `
            <h2 class="text-2xl font-bold text-gray-800 mb-6">Package Comparison</h2>
            <div class="overflow-x-auto">
                <table class="w-full text-left border-collapse">
                    <thead>
                        <tr class="border-b border-gray-300">
                            <th class="py-3 px-4 font-semibold text-gray-700 bg-gray-50">Property</th>
                            ${packages.map(pkg => `
                                <th class="py-3 px-4 font-semibold text-gray-700 bg-gray-50">${pkg.name}</th>
                            `).join('')}
                        </tr>
                    </thead>
                    <tbody>
                        <tr class="border-b border-gray-200">
                            <td class="py-3 px-4 text-gray-600 font-medium">Description</td>
                            ${packages.map(pkg => `
                                <td class="py-3 px-4 text-gray-700">${pkg.description || 'N/A'}</td>
                            `).join('')}
                        </tr>
                        <tr class="border-b border-gray-200 bg-gray-50">
                            <td class="py-3 px-4 text-gray-600 font-medium">License</td>
                            ${packages.map(pkg => `
                                <td class="py-3 px-4 text-gray-700">${pkg.license || 'N/A'}</td>
                            `).join('')}
                        </tr>
                        <tr class="border-b border-gray-200">
                            <td class="py-3 px-4 text-gray-600 font-medium">Language</td>
                            ${packages.map(pkg => `
                                <td class="py-3 px-4 text-gray-700">${pkg.language || 'N/A'}</td>
                            `).join('')}
                        </tr>
                        <tr class="border-b border-gray-200 bg-gray-50">
                            <td class="py-3 px-4 text-gray-600 font-medium">Platform</td>
                            ${packages.map(pkg => `
                                <td class="py-3 px-4 text-gray-700">${pkg.platform || 'N/A'}</td>
                            `).join('')}
                        </tr>
                        <tr class="border-b border-gray-200">
                            <td class="py-3 px-4 text-gray-600 font-medium">Created</td>
                            ${packages.map(pkg => `
                                <td class="py-3 px-4 text-gray-700">${new Date(pkg.created_at).toLocaleDateString()}</td>
                            `).join('')}
                        </tr>
                        <tr class="border-b border-gray-200 bg-gray-50">
                            <td class="py-3 px-4 text-gray-600 font-medium">Repository</td>
                            ${packages.map(pkg => `
                                <td class="py-3 px-4">
                                    ${pkg.repository ? `<a href="${pkg.repository}" target="_blank" class="text-blue-600 hover:underline">Link</a>` : 'N/A'}
                                </td>
                            `).join('')}
                        </tr>
                        <tr class="bg-gray-50">
                            <td class="py-3 px-4 text-gray-600 font-medium">Homepage</td>
                            ${packages.map(pkg => `
                                <td class="py-3 px-4">
                                    ${pkg.homepage ? `<a href="${pkg.homepage}" target="_blank" class="text-blue-600 hover:underline">Link</a>` : 'N/A'}
                                </td>
                            `).join('')}
                        </tr>
                    </tbody>
                </table>
            </div>
        `;

        showModal();
    } catch (error) {
        showNotification('Failed to load package comparison', 'error');
    }
}

async function showPackageDetails(packageId) {
    try {
        const response = await fetch(`/api/packages/${packageId}`);

        if (!response.ok) {
            showNotification('Failed to load package details', 'error');
            return;
        }

        const pkg = await response.json();

        const modalContent = document.getElementById('modal-content');
        modalContent.innerHTML = `
            <div class="space-y-6">
                <div>
                    <h2 class="text-3xl font-bold text-gray-800 mb-2">${pkg.name}</h2>
                    <p class="text-gray-600">${pkg.description || 'No description available'}</p>
                </div>

                ${pkg.tags && pkg.tags.length > 0 ? `
                    <div>
                        <h3 class="text-sm font-semibold text-gray-700 mb-2">Tags</h3>
                        <div class="flex flex-wrap gap-2">
                            ${pkg.tags.map(tag => `
                                <span class="bg-blue-100 text-blue-800 px-3 py-1 rounded-full text-sm">
                                    ${tag}
                                </span>
                            `).join('')}
                        </div>
                    </div>
                ` : ''}

                <div class="grid grid-cols-2 gap-4">
                    ${pkg.license ? `
                        <div>
                            <h3 class="text-sm font-semibold text-gray-700 mb-1">License</h3>
                            <p class="text-gray-600">${pkg.license}</p>
                        </div>
                    ` : ''}

                    ${pkg.platform ? `
                        <div>
                            <h3 class="text-sm font-semibold text-gray-700 mb-1">Platform</h3>
                            <p class="text-gray-600">${pkg.platform}</p>
                        </div>
                    ` : ''}

                    ${pkg.language ? `
                        <div>
                            <h3 class="text-sm font-semibold text-gray-700 mb-1">Language</h3>
                            <p class="text-gray-600">${pkg.language}</p>
                        </div>
                    ` : ''}

                    ${pkg.maintainers && pkg.maintainers.length > 0 ? `
                        <div>
                            <h3 class="text-sm font-semibold text-gray-700 mb-1">Maintainers</h3>
                            <p class="text-gray-600">${pkg.maintainers.join(', ')}</p>
                        </div>
                    ` : ''}
                </div>

                <div class="flex gap-4">
                    ${currentUser && authToken ? (
                        subscriptionsCache.has(pkg.name) ? `
                            <button onclick="unsubscribeFromPackage('${pkg.name}'); hideModal();"
                                    class="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors">
                                Unsubscribe
                            </button>
                        ` : `
                            <button onclick="subscribeToPackage('${pkg.name}'); hideModal();"
                                    class="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 transition-colors">
                                Subscribe
                            </button>
                        `
                    ) : ''}
                    ${pkg.homepage ? `
                        <a href="${pkg.homepage}" target="_blank"
                           class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">
                            Homepage
                        </a>
                    ` : ''}
                    ${pkg.repository ? `
                        <a href="${pkg.repository}" target="_blank"
                           class="px-4 py-2 bg-gray-700 text-white rounded-lg hover:bg-gray-800 transition-colors">
                            Repository
                        </a>
                    ` : ''}
                </div>

                <div class="text-xs text-gray-500 pt-4 border-t border-gray-200">
                    Created: ${new Date(pkg.created_at).toLocaleDateString()} |
                    Updated: ${new Date(pkg.updated_at).toLocaleDateString()}
                </div>
            </div>
        `;

        showModal();
    } catch (error) {
        showNotification('Failed to load package details', 'error');
    }
}

// Analytics functionality removed - use /api/analytics endpoint directly

// Subscriptions functionality
async function loadSubscriptions() {
    const subscriptionsContainer = document.getElementById('subscriptions-list');
    if (!subscriptionsContainer) return;

    // Show loading state
    subscriptionsContainer.innerHTML = `
        <div class="flex justify-center py-12">
            <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
        </div>
    `;

    try {
        const headers = {
            'Authorization': `Bearer ${authToken}`
        };

        const response = await fetch('/api/users/subscriptions', { headers });

        if (!response.ok) {
            throw new Error('Failed to load subscriptions');
        }

        const data = await response.json();
        await renderSubscriptions(data.subscriptions || [], subscriptionsContainer);
    } catch (error) {
        subscriptionsContainer.innerHTML = `
            <div class="text-center py-12">
                <div class="text-red-400 mb-4">
                    <svg class="w-16 h-16 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                    </svg>
                    <p class="text-lg font-medium">Error loading subscriptions</p>
                    <p class="text-sm text-gray-400 mt-2">Please try refreshing the page</p>
                </div>
            </div>
        `;
    }
}

async function renderSubscriptions(subscriptions, container) {
    if (subscriptions.length === 0) {
        container.innerHTML = `
            <div class="text-center py-12">
                <div class="text-gray-400 mb-4">
                    <svg class="w-16 h-16 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-4.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 009.586 13H7"></path>
                    </svg>
                    <p class="text-lg font-medium text-gray-300">No subscriptions yet</p>
                    <p class="text-sm text-gray-400 mt-2">Browse packages and subscribe to get updates</p>
                    <button onclick="showPackages()" class="mt-6 px-6 py-3 bg-blue-500 text-white rounded-lg hover:bg-blue-600 transition-colors">
                        Browse Packages
                    </button>
                </div>
            </div>
        `;
        return;
    }

    // Fetch package details for each subscription
    const packagePromises = subscriptions.map(async (packageName) => {
        try {
            const response = await fetch(`/api/packages?search=${encodeURIComponent(packageName)}`);
            if (!response.ok) return null;

            const data = await response.json();
            const pkg = data.packages?.find(p => p.name === packageName);
            return pkg || { name: packageName, description: 'Package details unavailable' };
        } catch (error) {
            return { name: packageName, description: 'Error loading package' };
        }
    });

    const packages = await Promise.all(packagePromises);

    const html = packages.filter(pkg => pkg !== null).map(pkg => `
        <div class="bg-gray-800 rounded-lg p-6 border border-gray-700 hover:border-gray-600 transition-colors">
            <div class="flex items-start justify-between">
                <div class="flex-1 min-w-0">
                    <h3 class="text-xl font-bold text-gray-100 mb-2 hover:text-blue-400 transition-colors cursor-pointer" onclick="showPackageDetails('${pkg.id || ''}')">${pkg.name}</h3>
                    <p class="text-gray-400 text-sm mb-4">${pkg.description || 'No description available'}</p>

                    <div class="flex flex-wrap gap-3 items-center">
                        ${pkg.license ? `
                            <span class="bg-green-500/10 text-green-400 px-3 py-1 rounded-full text-xs font-medium border border-green-500/20">
                                ${pkg.license}
                            </span>
                        ` : ''}
                        ${pkg.language ? `
                            <span class="bg-blue-500/10 text-blue-400 px-3 py-1 rounded-full text-xs font-medium border border-blue-500/20">
                                ${pkg.language}
                            </span>
                        ` : ''}
                        ${pkg.created_at ? `
                            <span class="text-xs text-gray-500">
                                Added ${new Date(pkg.created_at).toLocaleDateString()}
                            </span>
                        ` : ''}
                    </div>
                </div>

                <div class="ml-4 flex flex-col space-y-2">
                    <button onclick="unsubscribeFromPackage('${pkg.name}')" class="px-4 py-2 bg-red-500/10 text-red-400 border border-red-500/20 rounded-lg hover:bg-red-500/20 transition-colors text-sm font-medium">
                        Unsubscribe
                    </button>
                    ${pkg.repository ? `
                        <a href="${pkg.repository}" target="_blank" class="px-4 py-2 bg-gray-700 text-gray-300 rounded-lg hover:bg-gray-600 transition-colors text-sm font-medium text-center">
                            Repository
                        </a>
                    ` : ''}
                </div>
            </div>
        </div>
    `).join('');

    container.innerHTML = html;
}

async function subscribeToPackage(packageName) {
    if (!currentUser || !authToken) {
        showNotification('Please log in to subscribe to packages', 'warning');
        showLogin();
        return;
    }

    try {
        const response = await fetch('/api/users/subscriptions', {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${authToken}`,
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ package_name: packageName })
        });

        if (!response.ok) {
            throw new Error('Failed to subscribe');
        }

        subscriptionsCache.add(packageName);
        updateSubscriptionButtons(packageName);
        saveSubscriptionsToCache();
        showNotification(`Subscribed to ${packageName}`, 'success');
    } catch (error) {
        showNotification('Failed to subscribe. Please try again.', 'error');
    }
}

async function unsubscribeFromPackage(packageName) {
    if (!confirm(`Are you sure you want to unsubscribe from ${packageName}?`)) {
        return;
    }

    try {
        const response = await fetch(`/api/users/subscriptions/${encodeURIComponent(packageName)}`, {
            method: 'DELETE',
            headers: {
                'Authorization': `Bearer ${authToken}`
            }
        });

        if (!response.ok) {
            throw new Error('Failed to unsubscribe');
        }

        subscriptionsCache.delete(packageName);
        updateSubscriptionButtons(packageName);
        saveSubscriptionsToCache();
        showNotification(`Unsubscribed from ${packageName}`, 'success');

        // Reload subscriptions page if we're on it
        const subscriptionsPage = document.getElementById('subscriptions-page');
        if (subscriptionsPage && !subscriptionsPage.classList.contains('hidden')) {
            loadSubscriptions();
        }
    } catch (error) {
        showNotification('Failed to unsubscribe. Please try again.', 'error');
    }
}

// WebSocket functionality for real-time timeline updates
function connectTimelineWebSocket() {
    // Close existing connection if any
    if (timelineWebSocket) {
        timelineWebSocket.close();
        timelineWebSocket = null;
    }

    // Determine WebSocket URL
    // If running on localhost and frontend is on a different port, connect to backend port
    let wsUrl;
    if (window.location.hostname === 'localhost') {
        // Assume backend is on port 3000
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        wsUrl = `${protocol}//localhost:3000/ws/timeline`;
    } else {
        // Production: use same host as current page
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        wsUrl = `${protocol}//${window.location.host}/ws/timeline`;
    }

    console.log('Connecting to WebSocket:', wsUrl);
    timelineWebSocket = new WebSocket(wsUrl);

    timelineWebSocket.onopen = () => {
        console.log('Timeline WebSocket connected');

        // Send authentication if user is logged in
        if (authToken) {
            timelineWebSocket.send(JSON.stringify({
                type: 'Authenticate',
                token: authToken
            }));
        }
    };

    timelineWebSocket.onmessage = (event) => {
        try {
            const message = JSON.parse(event.data);

            switch (message.type) {
                case 'Authenticated':
                    console.log('Timeline WebSocket authenticated for user:', message.user_id);
                    break;

                case 'TimelineEvent':
                    // Add new event to timeline
                    addTimelineEventToUI(message.event);
                    break;

                case 'Ping':
                    // Respond to server ping
                    timelineWebSocket.send(JSON.stringify({ type: 'Pong' }));
                    break;

                case 'Pong':
                    // Server acknowledged our ping
                    break;
            }
        } catch (error) {
            console.error('Error parsing WebSocket message:', error);
        }
    };

    timelineWebSocket.onerror = (error) => {
        console.error('Timeline WebSocket error:', error);
    };

    timelineWebSocket.onclose = () => {
        console.log('Timeline WebSocket disconnected');
        timelineWebSocket = null;

        // Attempt to reconnect after 5 seconds if on timeline page
        const timelineSection = document.getElementById('timeline-section');
        if (timelineSection && !timelineSection.classList.contains('hidden')) {
            setTimeout(() => {
                console.log('Attempting to reconnect Timeline WebSocket...');
                connectTimelineWebSocket();
            }, 5000);
        }
    };
}

function disconnectTimelineWebSocket() {
    if (timelineWebSocket) {
        timelineWebSocket.close();
        timelineWebSocket = null;
    }
}

function addTimelineEventToUI(event) {
    const timelineContainer = document.getElementById('timeline-events');
    if (!timelineContainer) return;

    // Check if we've already displayed this event (by ID or by unique key)
    if (event.id && displayedEventIds.has(event.id)) {
        console.log('Skipping duplicate event:', event.id);
        return; // Skip duplicate
    }

    // Also check by a composite key (package_name + event_type + created_at)
    const eventKey = `${event.package_name}_${event.event_type}_${event.created_at}`;
    if (displayedEventIds.has(eventKey)) {
        console.log('Skipping duplicate event by key:', eventKey);
        return;
    }

    // Check if this is the empty state
    if (timelineContainer.querySelector('svg') && (
        timelineContainer.textContent.includes('Your timeline is empty') ||
        timelineContainer.textContent.includes('No activity yet') ||
        timelineContainer.textContent.includes('Waiting for activity')
    )) {
        // Replace empty state with the new event
        timelineContainer.innerHTML = '';
    }

    const eventDate = new Date(event.created_at);
    const timeAgo = getTimeAgo(eventDate);
    const title = getEventTitle(event);

    const eventHtml = `
        <div class="bg-gray-800 rounded-lg p-6 border border-gray-700 hover:border-gray-600 transition-colors animate-slideIn" data-event-id="${event.id || ''}">
            <div class="flex items-start space-x-4">
                <div class="flex-shrink-0">
                    ${getEventIcon(event.event_type)}
                </div>
                <div class="flex-1 min-w-0">
                    <div class="flex items-start justify-between">
                        <div>
                            <p class="text-gray-100 font-medium">${title}</p>
                            ${event.description ? `<p class="text-gray-400 text-sm mt-1">${event.description}</p>` : ''}
                            ${event.package_name ? `
                                <span class="inline-block mt-2 text-blue-400 text-sm hover:text-blue-300 cursor-pointer">
                                    ${event.package_name}
                                </span>
                            ` : ''}
                        </div>
                        <span class="text-xs text-gray-500 whitespace-nowrap ml-4 timeline-timestamp" data-timestamp="${event.created_at}">${timeAgo}</span>
                    </div>
                </div>
            </div>
        </div>
    `;

    // Prepend the new event to the top
    timelineContainer.insertAdjacentHTML('afterbegin', eventHtml);

    // Track this event ID for personal timeline
    if (currentUser && authToken && event.id) {
        displayedEventIds.add(event.id);
    }

    // For global timeline (logged-out users), keep only the 50 most recent events
    if (!currentUser || !authToken) {
        const events = timelineContainer.querySelectorAll('.bg-gray-800');
        if (events.length > 50) {
            // Remove events beyond the 50th
            for (let i = 50; i < events.length; i++) {
                events[i].remove();
            }
        }
    }
}

// Timeline functionality
async function loadTimeline(append = false) {
    const timelineContainer = document.getElementById('timeline-events');
    if (!timelineContainer) return;

    // Update title and subtitle based on auth state
    const titleElement = document.getElementById('timeline-title');
    const subtitleElement = document.getElementById('timeline-subtitle');
    if (currentUser && authToken) {
        if (titleElement) titleElement.textContent = 'Your Timeline';
        if (subtitleElement) subtitleElement.textContent = 'Activity from packages you\'re following';
    } else {
        if (titleElement) titleElement.textContent = 'Global Timeline';
        if (subtitleElement) subtitleElement.textContent = 'Latest activity across all packages';
    }

    // For global timeline (logged out), just show empty state and rely on WebSocket
    if (!currentUser || !authToken) {
        displayedEventIds.clear();
        timelineContainer.innerHTML = `
            <div class="text-center py-12">
                <div class="text-gray-400 mb-4">
                    <svg class="w-16 h-16 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                    </svg>
                    <p class="text-lg font-medium text-gray-300">Waiting for activity...</p>
                    <p class="text-sm text-gray-400 mt-2">Events will appear here as they happen</p>
                </div>
            </div>
        `;
        // Connect to WebSocket for real-time updates
        connectTimelineWebSocket();
        // Start updating timestamps
        startTimelineUpdates();
        return;
    }

    // Personal timeline - load from API with pagination
    if (timelineLoading) return;
    timelineLoading = true;

    // Reset offset if not appending
    if (!append) {
        timelineOffset = 0;
        displayedEventIds.clear();
    }

    // Show loading state only if not appending
    if (!append) {
        timelineContainer.innerHTML = `
            <div class="flex justify-center py-12">
                <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
            </div>
        `;
    }

    try {
        const headers = {
            'Authorization': `Bearer ${authToken}`
        };

        // Build URL with pagination params
        const url = `/api/users/timeline?limit=${timelineLimit}&offset=${timelineOffset}`;

        const response = await fetch(url, { headers });

        if (!response.ok) {
            throw new Error('Failed to load timeline');
        }

        const data = await response.json();

        if (data.total !== undefined) {
            timelineTotal = data.total;
        }

        if (append) {
            appendTimelineEvents(data.events || [], timelineContainer);
        } else {
            renderTimeline(data.events || [], timelineContainer);
        }

        // Track displayed event IDs
        (data.events || []).forEach(event => {
            if (event.id) displayedEventIds.add(event.id);
        });

        // Update offset for next page
        timelineOffset += (data.events || []).length;

        // Connect to WebSocket for real-time updates (only once)
        if (!append) {
            connectTimelineWebSocket();
            // Start updating timestamps
            startTimelineUpdates();
        }
    } catch (error) {
        if (!append) {
            timelineContainer.innerHTML = `
                <div class="text-center py-12">
                    <div class="text-red-400 mb-4">
                        <svg class="w-16 h-16 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                        </svg>
                        <p class="text-lg font-medium">Error loading timeline</p>
                        <p class="text-sm text-gray-400 mt-2">Please try refreshing the page</p>
                    </div>
                </div>
            `;
        }
    } finally {
        timelineLoading = false;
    }
}

function renderTimeline(events, container) {
    if (events.length === 0) {
        const emptyMessage = currentUser && authToken
            ? {
                title: 'Your timeline is empty',
                subtitle: 'Subscribe to packages to see updates here'
              }
            : {
                title: 'No activity yet',
                subtitle: 'Check back later for updates'
              };

        container.innerHTML = `
            <div class="text-center py-12">
                <div class="text-gray-400 mb-4">
                    <svg class="w-16 h-16 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                    </svg>
                    <p class="text-lg font-medium text-gray-300">${emptyMessage.title}</p>
                    <p class="text-sm text-gray-400 mt-2">${emptyMessage.subtitle}</p>
                </div>
            </div>
        `;
        return;
    }

    const html = events.map(event => {
        const eventDate = new Date(event.created_at);
        const timeAgo = getTimeAgo(eventDate);
        const title = getEventTitle(event);

        return `
            <div class="bg-gray-800 rounded-lg p-6 border border-gray-700 hover:border-gray-600 transition-colors">
                <div class="flex items-start space-x-4">
                    <div class="flex-shrink-0">
                        ${getEventIcon(event.event_type)}
                    </div>
                    <div class="flex-1 min-w-0">
                        <div class="flex items-start justify-between">
                            <div>
                                <p class="text-gray-100 font-medium">${title}</p>
                                ${event.description ? `<p class="text-gray-400 text-sm mt-1">${event.description}</p>` : ''}
                                ${event.package_name ? `
                                    <span class="inline-block mt-2 text-blue-400 text-sm hover:text-blue-300 cursor-pointer">
                                        ${event.package_name}
                                    </span>
                                ` : ''}
                            </div>
                            <span class="text-xs text-gray-500 whitespace-nowrap ml-4 timeline-timestamp" data-timestamp="${event.created_at}">${timeAgo}</span>
                        </div>
                    </div>
                </div>
            </div>
        `;
    }).join('');

    container.innerHTML = html;

    // Add "Load More" button for personal timeline
    if (currentUser && authToken && timelineOffset < timelineTotal) {
        const loadMoreHtml = `
            <div class="flex justify-center py-6">
                <button onclick="loadTimeline(true)" class="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">
                    Load More (${timelineTotal - timelineOffset} remaining)
                </button>
            </div>
        `;
        container.insertAdjacentHTML('beforeend', loadMoreHtml);
    }
}

function appendTimelineEvents(events, container) {
    if (events.length === 0) return;

    // Remove existing "Load More" button if present
    const loadMoreBtn = container.querySelector('button[onclick*="loadTimeline"]');
    if (loadMoreBtn) {
        loadMoreBtn.parentElement.remove();
    }

    const html = events.map(event => {
        const eventDate = new Date(event.created_at);
        const timeAgo = getTimeAgo(eventDate);
        const title = getEventTitle(event);

        return `
            <div class="bg-gray-800 rounded-lg p-6 border border-gray-700 hover:border-gray-600 transition-colors">
                <div class="flex items-start space-x-4">
                    <div class="flex-shrink-0">
                        ${getEventIcon(event.event_type)}
                    </div>
                    <div class="flex-1 min-w-0">
                        <div class="flex items-start justify-between">
                            <div>
                                <p class="text-gray-100 font-medium">${title}</p>
                                ${event.description ? `<p class="text-gray-400 text-sm mt-1">${event.description}</p>` : ''}
                                ${event.package_name ? `
                                    <span class="inline-block mt-2 text-blue-400 text-sm hover:text-blue-300 cursor-pointer">
                                        ${event.package_name}
                                    </span>
                                ` : ''}
                            </div>
                            <span class="text-xs text-gray-500 whitespace-nowrap ml-4 timeline-timestamp" data-timestamp="${event.created_at}">${timeAgo}</span>
                        </div>
                    </div>
                </div>
            </div>
        `;
    }).join('');

    container.insertAdjacentHTML('beforeend', html);

    // Add new "Load More" button if there are more events
    if (currentUser && authToken && timelineOffset < timelineTotal) {
        const loadMoreHtml = `
            <div class="flex justify-center py-6">
                <button onclick="loadTimeline(true)" class="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors">
                    Load More (${timelineTotal - timelineOffset} remaining)
                </button>
            </div>
        `;
        container.insertAdjacentHTML('beforeend', loadMoreHtml);
    }
}

function getEventTitle(event) {
    switch(event.event_type) {
        case 'PackageAdded':
            return event.package_name;
        case 'NewRelease':
            return `${event.package_name} ${event.version || ''}`.trim();
        case 'SecurityAlert':
            return `Security Alert: ${event.package_name}`;
        case 'PackageUpdated':
            return `${event.package_name} Updated`;
        default:
            return event.package_name || 'Update';
    }
}

function getEventIcon(eventType) {
    const iconClasses = "w-10 h-10 p-2 rounded-full";

    switch(eventType) {
        case 'PackageAdded':
            return `
                <div class="${iconClasses} bg-green-500/10 text-green-400">
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                    </svg>
                </div>
            `;
        case 'NewRelease':
            return `
                <div class="${iconClasses} bg-blue-500/10 text-blue-400">
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"></path>
                    </svg>
                </div>
            `;
        case 'SecurityAlert':
            return `
                <div class="${iconClasses} bg-red-500/10 text-red-400">
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
                    </svg>
                </div>
            `;
        case 'PackageUpdated':
            return `
                <div class="${iconClasses} bg-purple-500/10 text-purple-400">
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
                    </svg>
                </div>
            `;
        default:
            return `
                <div class="${iconClasses} bg-gray-500/10 text-gray-400">
                    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                    </svg>
                </div>
            `;
    }
}

function getTimeAgo(date) {
    const seconds = Math.floor((new Date() - date) / 1000);

    const intervals = {
        year: 31536000,
        month: 2592000,
        week: 604800,
        day: 86400,
        hour: 3600,
        minute: 60
    };

    for (const [unit, secondsInUnit] of Object.entries(intervals)) {
        const interval = Math.floor(seconds / secondsInUnit);
        if (interval >= 1) {
            return `${interval} ${unit}${interval !== 1 ? 's' : ''} ago`;
        }
    }

    return 'just now';
}

function updateTimelineTimestamps() {
    const timestamps = document.querySelectorAll('.timeline-timestamp');
    timestamps.forEach(span => {
        const timestamp = span.getAttribute('data-timestamp');
        if (timestamp) {
            const date = new Date(timestamp);
            span.textContent = getTimeAgo(date);
        }
    });
}

function startTimelineUpdates() {
    // Clear any existing interval
    if (timelineUpdateInterval) {
        clearInterval(timelineUpdateInterval);
    }

    // Update timestamps every 30 seconds
    timelineUpdateInterval = setInterval(updateTimelineTimestamps, 30000);
}

function stopTimelineUpdates() {
    if (timelineUpdateInterval) {
        clearInterval(timelineUpdateInterval);
        timelineUpdateInterval = null;
    }
}

// Load saved preferences
document.addEventListener('DOMContentLoaded', () => {
    // Load view mode preference
    const savedViewMode = localStorage.getItem('viewMode');
    if (savedViewMode) {
        currentViewMode = savedViewMode;
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

    .animate-slideIn {
        animation: slideIn 0.5s ease-out forwards;
    }

    @keyframes slideIn {
        from {
            opacity: 0;
            transform: translateY(-20px);
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

    .animate-shake {
        animation: shake 0.6s ease-in-out;
    }

    @keyframes shake {
        0%, 100% { transform: translateX(0); }
        10%, 30%, 50%, 70%, 90% { transform: translateX(-8px); }
        20%, 40%, 60%, 80% { transform: translateX(8px); }
    }
`;
document.head.appendChild(style);