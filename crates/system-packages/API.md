```yaml
system:
  dependencies:
    # Same across everything
    - foo
    # By OS
    - name: libfoo
      os: linux
    # By package manager
    - name: foo
      manager: brew
    - name: libfoo
      manager: yum
    # By anything matching name
    - name:
        brew: foo
        yum: libfoo-sys
        linux: libfoo
    # With advanced settings?
    - name: foo
      version: "1.2.3"
      os: linux
      arch: "..."
      optional: true
      env:
        CFLAGS: "..."
        CC: "..."
```
