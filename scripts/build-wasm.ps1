# scripts/build-wasm.ps1
# Windows PowerShell version of the WASM build script.

Write-Host "Building LabWise WASM bridge..." -ForegroundColor Cyan

Push-Location "$PSScriptRoot\..\bridge"

try {
    wasm-pack build --target web --out-dir ../projection/src/wasm-pkg
    Write-Host "WASM build complete -> projection/src/wasm-pkg/" -ForegroundColor Green
}
catch {
    Write-Host "WASM build failed!" -ForegroundColor Red
    throw
}
finally {
    Pop-Location
}
