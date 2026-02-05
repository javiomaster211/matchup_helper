# Build script for MatchupHelper
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

Write-Host "Building MatchupHelper..."
npm run tauri build

if ($LASTEXITCODE -eq 0) {
    Write-Host "Build completed successfully!"
    Write-Host "Executable: src-tauri\target\release\matchuphelper.exe"
    Get-ChildItem src-tauri\target\release\*.exe | ForEach-Object {
        Write-Host "Size: $([math]::Round($_.Length / 1MB, 2)) MB"
    }
} else {
    Write-Host "Build failed with exit code $LASTEXITCODE"
}
