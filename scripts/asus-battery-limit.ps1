# Set ASUS / ROG battery charge limit via WMI (AsusAtkWmi_WMNB) or ATKACPI when available.
param(
    [Parameter(Mandatory = $true)]
    [ValidateRange(1, 100)]
    [int]$Percent
)

$ErrorActionPreference = "Stop"

function Set-LimitWmi {
    $wmi = Get-WmiObject -Namespace root/WMI -Class AsusAtkWmi_WMNB -ErrorAction Stop
    $null = $wmi.DEVS(0x00120057, $Percent)
}

function Set-LimitIoctl {
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class AtkAcpi {
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    public static extern IntPtr CreateFileW(string lpFileName, uint dwDesiredAccess, uint dwShareMode,
        IntPtr lpSecurityAttributes, uint dwCreationDisposition, uint dwFlagsAndAttributes, IntPtr hTemplateFile);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern bool DeviceIoControl(IntPtr hDevice, uint dwIoControlCode, byte[] lpInBuffer, uint nInBufferSize,
        IntPtr lpOutBuffer, uint nOutBufferSize, out uint lpBytesReturned, IntPtr lpOverlapped);
    [DllImport("kernel32.dll", SetLastError=true)]
    public static extern bool CloseHandle(IntPtr hObject);
    public const uint GENERIC_READ = 0x80000000;
    public const uint GENERIC_WRITE = 0x40000000;
    public const uint OPEN_EXISTING = 3;
    public const uint IOCTL = 0x0022240C;
    public static void SetLimit(int percent) {
        IntPtr h = CreateFileW(@"\\.\\ATKACPI", GENERIC_READ | GENERIC_WRITE, 3, IntPtr.Zero, OPEN_EXISTING, 0, IntPtr.Zero);
        if (h == IntPtr.Zero || h.ToInt64() == -1) throw new System.ComponentModel.Win32Exception();
        byte[] buf = new byte[8];
        BitConverter.GetBytes((uint)0x00120057).CopyTo(buf, 0);
        BitConverter.GetBytes((uint)percent).CopyTo(buf, 4);
        uint ret;
        if (!DeviceIoControl(h, IOCTL, buf, 8, IntPtr.Zero, 0, out ret, IntPtr.Zero))
            throw new System.ComponentModel.Win32Exception();
        CloseHandle(h);
    }
}
"@ -ErrorAction SilentlyContinue
    [AtkAcpi]::SetLimit($Percent)
}

try {
    Set-LimitIoctl
    Write-Output "ok:ioctl:$Percent"
    exit 0
} catch {
    Write-Verbose "IOCTL failed: $_"
}

try {
    Set-LimitWmi
    Write-Output "ok:wmi:$Percent"
    exit 0
} catch {
    Write-Error "Failed to set ASUS charge limit to $Percent%: $_"
    exit 1
}
