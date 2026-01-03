# Security Policy

## Supported Versions

Currently only the latest version is supported.

## Reporting a Vulnerability

If you discover a security vulnerability, please report it privately.

### How to Report

Send email to: security@ cathedralfabric.dev (placeholder)

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if known)

### Response Timeline

- Initial response: Within 48 hours
- Detailed response: Within 7 days
- Fix timeline: Depends on severity

### What to Expect

1. We will acknowledge receipt of your report
2. We will investigate the vulnerability
3. We will work on a fix
4. We will coordinate disclosure with you
5. We will credit you (if desired)

## Security Best Practices

### For Deployments

1. **Keep updated** - Run latest version
2. **Restrict network access** - Use firewalls
3. **Enable authentication** - In cluster mode
4. **Review policies** - Regularly audit
5. **Monitor logs** - For suspicious activity
6. **Encrypt in transit** - Use TLS
7. **Encrypt at rest** - For sensitive data
8. **Principle of least privilege** - Minimal capabilities

### For Development

1. **No unsafe without audit** - All `unsafe` reviewed
2. **Fuzz critical code** - Parsers, encoders
3. **Property tests** - For invariants
4. **Security reviews** - For major changes
5. **Dependency audits** - Regular updates

## Security-Related Files

- [SECURITY.md](docs/SECURITY.md) - Full security model
- [FAILURE_MODES.md](docs/FAILURE_MODES.md) - Failure handling
- [CAPABILITIES.md](docs/CAPABILITIES.md) - Capability system
- [POLICY.md](docs/POLICY.md) - Policy language

## Public Disclosure

Security advisories will be:
1. Published as GitHub Security Advisories
2. Included in release notes
3. Added to CHANGELOG.md

Credits for vulnerability reporters included (with permission).

## Security Audits

Professional security audits will be:
1. Announced in advance
2. Conducted by reputable firms
3. Findings addressed promptly
4. Reports published (with sensitive details redacted)
