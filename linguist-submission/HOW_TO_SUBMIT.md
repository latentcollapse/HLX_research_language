# How to Submit HLX to GitHub Linguist

## Prerequisites

1. Fork the repository: https://github.com/github/linguist
2. Clone your fork locally
3. Create a new branch: `git checkout -b add-hlx-language`

## Steps

### 1. Add Language Definition

Edit `lib/linguist/languages.yml` and add the HLX entry (alphabetically sorted):

```yaml
HLX:
  type: programming
  color: "#9d4edd"
  aliases:
    - hlx
  extensions:
    - ".hlxa"
    - ".hlxc"
  tm_scope: source.hlx
  ace_mode: text
  interpreters:
    - hlx
```

### 2. Add TextMate Grammar

Copy `hlx.tmLanguage.json` to `grammars/source.hlx.tmLanguage.json` in the Linguist repo.

### 3. Add Sample File

Copy `sample.hlxa` to `samples/HLX/sample.hlxa` in the Linguist repo.

### 4. Run Tests

```bash
bundle install
bundle exec rake samples
bundle exec rake test
```

### 5. Create Pull Request

Commit your changes:
```bash
git add .
git commit -m "Add HLX language support"
git push origin add-hlx-language
```

Create PR on GitHub with description:

**Title**: Add HLX language support

**Description**:
```
Adds support for HLX (Helix Language), a self-hosting deterministic programming language.

- Extensions: .hlxa, .hlxc
- Repository: https://github.com/latentcollapse/hlx-compiler
- Status: Self-hosting compiler with bootstrap verification
- License: Apache 2.0

This PR adds:
- Language definition in languages.yml
- TextMate grammar for syntax highlighting
- Sample code for testing
```

## Linguist Requirements Checklist

- [ ] Language is actively maintained (repo shows recent commits)
- [ ] Language has clear documentation
- [ ] Language has working compiler/interpreter
- [ ] Sample code provided
- [ ] TextMate grammar included
- [ ] Tests pass locally

## Expected Timeline

- PR review: 1-2 weeks
- Merge: After maintainer approval
- Deployment: Next Linguist release (varies)
- GitHub adoption: After deployment, within days

## Notes

- Be responsive to maintainer feedback
- Be prepared to adjust grammar or samples if requested
- Language ID will be assigned by maintainers (remove from submission)
