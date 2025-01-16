# Introduction

A D-BUS service that allows to perform firmware upgrade for BeagleBoard devices (specifically Pocketbeagle2 MSPM0), from userspace applications (GUI).

This is needed because there is no existing D-BUS service (like UDisk2) to open root sysfs entries as a non root user/application. Additionally, it seems most distributions heavily discourage use of `pkexec` since [CVE-2021-4034](https://blog.qualys.com/vulnerabilities-threat-research/2022/01/25/pwnkit-local-privilege-escalation-vulnerability-discovered-in-polkits-pkexec-cve-2021-4034).

It uses polkit to verify if the application has necessary permissions to perform firmware upgrade.

## Dependencies

- polkit
- systemd

It is possible to make things work without systemd, but it is not officially supported right now.
