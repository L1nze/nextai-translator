$procs = Get-CimInstance Win32_Process -Filter "Name='node.exe'" | Where-Object { $_.CommandLine -match 'vitest' }
if (-not $procs) {
    Write-Output 'NO_VITEST_PROCESSES'
} else {
    foreach ($p in $procs) {
        $cmd = $p.CommandLine
        if ($cmd -and $cmd.Length -gt 200) { $cmd = $cmd.Substring(0, 200) }
        Write-Output ("PID={0} CMD={1}" -f $p.ProcessId, $cmd)
    }
}
