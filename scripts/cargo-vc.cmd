@echo off
rem 在 VS Build Tools 环境中执行 cargo 命令（解决 Git Bash 下 GNU link 抢占 MSVC link 的问题）
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=amd64 -no_logo
set PATH=%USERPROFILE%\.cargo\bin;%PATH%
cd /d E:\WorkSpace\zai\edge-todo\src-tauri
cargo %*
