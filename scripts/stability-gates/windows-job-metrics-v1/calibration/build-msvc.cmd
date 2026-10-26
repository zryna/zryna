@echo off
setlocal
if "%~1"=="" exit /b 2
if not "%~2"=="" exit /b 2
if not "%~1"=="%~f1" exit /b 2
if exist "%~1" exit /b 2
mkdir "%~1" || exit /b 2
set "FX_BUILD_DIR=%~1"
set "CL="
set "_CL_="
set "LINK="
set "_LINK_="
pushd "%~dp0" || exit /b 2
cl /nologo /std:c17 /W4 /WX /O2 /MT /GS /guard:cf /Brepro /utf-8 /DUNICODE /D_UNICODE /D_WIN32_WINNT=0x0A00 /Fo"%FX_BUILD_DIR%\\" /Fe"%FX_BUILD_DIR%\windows-job-calibration-v1.exe" main.c control.c launch.c subject.c observe.c supervisor.c cleanup.c report.c ..\portable.c /link /Brepro /NXCOMPAT /DYNAMICBASE kernel32.lib
set "FX_BUILD_EXIT=%ERRORLEVEL%"
popd
exit /b %FX_BUILD_EXIT%
