/// Check if a license string represents a free/open source license
pub fn is_free_license(license: &str) -> bool {
    let normalized = license.to_lowercase();

    let free_licenses = [
        "mit", "apache", "apache-2.0", "apache 2.0", "bsd", "isc", "cc0",
        "unlicense", "wtfpl", "0bsd", "bsl-1.0", "ncsa", "zlib", "x11",
        "gpl", "lgpl", "agpl", "mpl", "epl", "cpl", "cddl", "cecill",
        "eupl", "osl", "afl", "artistic",
        "cc-by", "cc-by-sa",
        "public domain", "publicdomain", "unlicensed",
    ];

    let non_free_keywords = [
        "proprietary", "commercial", "private", "closed",
        "all rights reserved", "copyright only",
        "cc-by-nd", "cc-by-nc",
    ];

    for keyword in &non_free_keywords {
        if normalized.contains(keyword) {
            return false;
        }
    }

    for free_license in &free_licenses {
        if normalized.contains(free_license) {
            return true;
        }
    }

    if normalized.contains(" or ") || normalized.contains("/") {
        let parts: Vec<&str> = normalized.split(&[' ', '/', '|'][..]).collect();
        for part in parts {
            let part = part.trim();
            if part.is_empty() || part == "or" {
                continue;
            }
            for free_license in &free_licenses {
                if part.contains(free_license) {
                    return true;
                }
            }
        }
    }

    tracing::warn!("Unknown license, treating as non-free: {}", license);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_free_license() {
        assert!(is_free_license("MIT"));
        assert!(is_free_license("Apache-2.0"));
        assert!(is_free_license("BSD-3-Clause"));
        assert!(is_free_license("GPL-3.0"));
        assert!(is_free_license("MIT OR Apache-2.0"));
        assert!(!is_free_license("proprietary"));
        assert!(!is_free_license("CC-BY-NC"));
        assert!(!is_free_license("CustomLicense"));
    }
}
