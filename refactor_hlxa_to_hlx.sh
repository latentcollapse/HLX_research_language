#!/bin/bash
# HLX File Extension Refactor: .hlx → .hlx
# Reason: Runic (HLX-R) is now a separate project, no need for 'a' suffix

set -e

echo "════════════════════════════════════════════════════════════"
echo "HLX Extension Refactor: .hlx → .hlx"
echo "════════════════════════════════════════════════════════════"
echo ""

# 1. Create backup
BACKUP_DIR="/home/matt/hlx-compiler-backup-$(date +%Y%m%d_%H%M%S)"
echo "[1/5] Creating backup at $BACKUP_DIR..."
cp -r /home/matt/hlx-compiler "$BACKUP_DIR"
echo "✅ Backup created"
echo ""

# 2. Rename all .hlx files to .hlx
echo "[2/5] Renaming .hlx files to .hlx..."
cd /home/matt/hlx-compiler
find . -name "*.hlx" -type f | while read file; do
    newfile="${file%.hlx}.hlx"
    mv "$file" "$newfile"
    echo "  Renamed: $file → $newfile"
done
echo "✅ Files renamed"
echo ""

# 3. Update references in documentation
echo "[3/5] Updating references in .md files..."
find . -name "*.md" -type f -exec sed -i 's/\.hlx/.hlx/g' {} \;
echo "✅ Documentation updated"
echo ""

# 4. Update references in source files
echo "[4/5] Updating references in .hlx source files..."
find . -name "*.hlx" -type f -exec sed -i 's/\.hlx/.hlx/g' {} \;
echo "✅ Source files updated"
echo ""

# 5. Update references in scripts
echo "[5/5] Updating references in shell scripts..."
find . -name "*.sh" -type f -exec sed -i 's/\.hlx/.hlx/g' {} \;
echo "✅ Shell scripts updated"
echo ""

echo "════════════════════════════════════════════════════════════"
echo "Refactor Complete!"
echo "════════════════════════════════════════════════════════════"
echo ""
echo "Summary:"
echo "  • All .hlx files renamed to .hlx"
echo "  • Documentation updated"
echo "  • Source references updated"
echo "  • Backup saved at: $BACKUP_DIR"
echo ""
echo "Next steps:"
echo "  1. Update RustD compiler to recognize .hlx extension"
echo "  2. Test compilation with new extension"
echo "  3. Commit changes to git"
echo ""
