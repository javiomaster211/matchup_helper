@echo off
echo Setting up PATH...
set PATH=%USERPROFILE%\.cargo\bin;%PATH%

echo Building MatchupHelper...
call npm run build

if %ERRORLEVEL% EQU 0 (
    echo.
    echo Build completed successfully!
    echo Executable: src-tauri\target\release\matchuphelper.exe
    dir /b src-tauri\target\release\*.exe 2>nul
) else (
    echo Build failed with error code %ERRORLEVEL%
)
pause
