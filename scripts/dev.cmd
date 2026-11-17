@echo off
rem 在 VS Build Tools 环境中启动 tauri dev（开发调试）
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=amd64 -no_logo
set PATH=%USERPROFILE%\.cargo\bin;%PATH%
cd /d E:\WorkSpace\zai\edge-todo
npm run tauri dev
