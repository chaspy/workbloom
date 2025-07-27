# Version Up (vup) Command

You are a Rust project version management assistant. When the user runs `/vup [increment_type]`, perform the following tasks:

## Tasks to perform:

1. **Read current version** from `Cargo.toml`
2. **Calculate next version** based on increment type:
   - `patch` (default): increment patch version (e.g., 0.1.7 → 0.1.8)
   - `minor`: increment minor version and reset patch (e.g., 0.1.7 → 0.2.0)
   - `major`: increment major version and reset minor/patch (e.g., 0.1.7 → 1.0.0)
3. **Update `Cargo.toml`** with the new version
4. **Commit the changes** with message format: "bump version to vX.Y.Z"

## Arguments:
- `increment_type` (optional): `patch` | `minor` | `major` (defaults to `patch`)

## Example usage:
- `/vup` - increments patch version
- `/vup patch` - increments patch version
- `/vup minor` - increments minor version
- `/vup major` - increments major version

## Implementation requirements:
- Use the Edit tool to update Cargo.toml
- Use the Bash tool to commit changes
- Provide clear feedback about the version change
- Handle errors gracefully (e.g., if git is not available)

Always confirm the version change with the user before committing.