#!/usr/bin/env bash
# Fast GPU stats for Waybar (RTX 3050 Laptop)

IFS=', ' read -r util temp used total < <(nvidia-smi --query-gpu=utilization.gpu,temperature.gpu,memory.used,memory.total --format=csv,noheader,nounits 2>/dev/null)
if [ -n "$util" ]; then
    printf '{"text":"󰢮 %s%% %s°C","tooltip":"NVIDIA GeForce RTX 3050\\nUsage: %s%%\\nTemp: %s°C\\nVRAM: %sMiB / %sMiB"}\n' "$util" "$temp" "$util" "$temp" "$used" "$total"
else
    printf '{"text":"󰢮 N/A","tooltip":"GPU not found"}\n'
fi
