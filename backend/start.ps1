$envFile = Join-Path $PSScriptRoot ".env"
Get-Content $envFile | ForEach-Object {
    if ($_ -match '^([^#=]+)=(.*)$') {
        [System.Environment]::SetEnvironmentVariable($Matches[1].Trim(), $Matches[2].Trim())
    }
}
& "$PSScriptRoot\target\release\server.exe"
