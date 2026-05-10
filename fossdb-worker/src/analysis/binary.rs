use anyhow::Result;
use fossdb::{AnalysisFinding, FindingKind, FindingSeverity};
use std::path::Path;

pub fn analyze(path: &Path) -> Result<Vec<AnalysisFinding>> {
    let bytes = std::fs::read(path)?;
    let mut findings = analyze_bytes(&bytes, path.to_string_lossy().as_ref())?;

    #[cfg(feature = "analysis-symbolic")]
    {
        match super::symbolic::extract_syscalls_program(path) {
            Ok(syscalls) => {
                findings.append(&mut super::symbolic::syscalls_to_findings(
                    &syscalls,
                    path.to_string_lossy().as_ref(),
                ));
            }
            Err(e) => tracing::debug!("whole-program symbolic analysis skipped: {}", e),
        }
    }

    Ok(findings)
}

pub fn analyze_bytes(bytes: &[u8], name: &str) -> Result<Vec<AnalysisFinding>> {
    #[cfg(feature = "analysis-binary")]
    {
        use goblin::Object;
        match Object::parse(bytes)? {
            Object::Elf(elf) => analyze_elf(&elf, bytes, name),
            Object::PE(pe) => analyze_pe(&pe, name),
            Object::Mach(goblin::mach::Mach::Binary(macho)) => analyze_macho(&macho, name),
            _ => Ok(vec![]),
        }
    }
    #[cfg(not(feature = "analysis-binary"))]
    {
        let _ = (bytes, name);
        Ok(vec![])
    }
}

#[cfg(feature = "analysis-binary")]
fn analyze_elf(elf: &goblin::elf::Elf, bytes: &[u8], name: &str) -> Result<Vec<AnalysisFinding>> {
    let mut findings = Vec::new();

    findings.push(AnalysisFinding {
        kind: FindingKind::BinaryMetadata,
        severity: FindingSeverity::Info,
        description: format!(
            "ELF binary: arch={}, is_64={}, stripped={}",
            elf.header.e_machine,
            elf.is_64,
            elf.syms.is_empty()
        ),
        location: Some(name.to_string()),
    });

    let dangerous_symbols = [
        "system", "execve", "execvp", "execl", "popen", "dlopen",
        "ptrace", "mprotect", "mmap",
    ];

    let flagged: Vec<&str> = elf
        .syms
        .iter()
        .filter_map(|sym| elf.strtab.get_at(sym.st_name))
        .filter(|name| dangerous_symbols.iter().any(|d| name.contains(d)))
        .collect();

    if !flagged.is_empty() {
        findings.push(AnalysisFinding {
            kind: FindingKind::MaliciousPattern,
            severity: FindingSeverity::Low,
            description: format!("Potentially sensitive symbols: {}", flagged.join(", ")),
            location: Some(name.to_string()),
        });
    }

    // Check for packed/obfuscated sections: very high entropy sections
    for section in &elf.section_headers {
        if section.sh_size > 0 && section.sh_type == goblin::elf::section_header::SHT_PROGBITS {
            let start = section.sh_offset as usize;
            let end = (start + section.sh_size as usize).min(bytes.len());
            if end > start {
                let entropy = byte_entropy(&bytes[start..end]);
                if entropy > 7.5 {
                    let section_name = elf
                        .shdr_strtab
                        .get_at(section.sh_name)
                        .unwrap_or("unknown");
                    findings.push(AnalysisFinding {
                        kind: FindingKind::MaliciousPattern,
                        severity: FindingSeverity::Medium,
                        description: format!(
                            "High-entropy section '{}' (entropy={:.2}), possible packing or encryption",
                            section_name, entropy
                        ),
                        location: Some(name.to_string()),
                    });
                }
            }
        }
    }

    Ok(findings)
}

#[cfg(feature = "analysis-binary")]
fn analyze_pe(pe: &goblin::pe::PE, name: &str) -> Result<Vec<AnalysisFinding>> {
    let mut findings = Vec::new();

    findings.push(AnalysisFinding {
        kind: FindingKind::BinaryMetadata,
        severity: FindingSeverity::Info,
        description: format!(
            "PE binary: is_64={}, is_lib={}",
            pe.is_64, pe.is_lib
        ),
        location: Some(name.to_string()),
    });

    let suspicious_imports = ["CreateRemoteThread", "VirtualAllocEx", "WriteProcessMemory", "LoadLibrary"];
    let flagged: Vec<String> = pe
        .imports
        .iter()
        .filter(|imp| suspicious_imports.iter().any(|s| imp.name.contains(*s)))
        .map(|imp| imp.name.to_string())
        .collect();

    if !flagged.is_empty() {
        findings.push(AnalysisFinding {
            kind: FindingKind::MaliciousPattern,
            severity: FindingSeverity::Medium,
            description: format!("Suspicious PE imports: {}", flagged.join(", ")),
            location: Some(name.to_string()),
        });
    }

    Ok(findings)
}

#[cfg(feature = "analysis-binary")]
fn analyze_macho(macho: &goblin::mach::MachO, name: &str) -> Result<Vec<AnalysisFinding>> {
    let mut findings = Vec::new();

    findings.push(AnalysisFinding {
        kind: FindingKind::BinaryMetadata,
        severity: FindingSeverity::Info,
        description: format!("Mach-O binary: {} segments", macho.segments.len()),
        location: Some(name.to_string()),
    });

    Ok(findings)
}

fn byte_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut counts = [0u64; 256];
    for &b in data {
        counts[b as usize] += 1;
    }
    let len = data.len() as f64;
    counts.iter().filter(|&&c| c > 0).fold(0.0, |acc, &c| {
        let p = c as f64 / len;
        acc - p * p.log2()
    })
}
