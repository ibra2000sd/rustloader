# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability in Rustloader, please report it responsibly:

1. **DO NOT** open a public issue
2. Email: security@example.com
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

## Response Timeline

- **Initial Response**: Within 48 hours
- **Assessment**: Within 7 days
- **Fix Timeline**: Depends on severity
  - Critical: 24-48 hours
  - High: 7 days
  - Medium: 30 days
  - Low: Next release

## Security Measures

This project implements:
- Automated dependency auditing (cargo-audit)
- License compliance checking (cargo-deny)
- Supply chain security (cargo-vet)
- Path traversal protection in file handling
- Input sanitization for filenames

## Known Issues

See [KNOWN_ISSUES.md](KNOWN_ISSUES.md) for current security-related limitations.
