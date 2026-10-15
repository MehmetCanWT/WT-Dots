#!/usr/bin/env bash
# Fast GPU stats for Waybar (RTX 3050 Laptop)

IFS=', ' read -r util temp used total < <(nvidia-smi --query-gpu=utilization.gpu,temperature.gpu,memory.used,memory.total --format=csv,noheader,nounits 2>/dev/null)
if [ -n "$util" ]; then
    printf '{"text":"󰢮 %s%%","tooltip":"NVIDIA GeForce RTX 3050\\nKullanım: %s%%\\nSıcaklık: %s°C\\nVRAM: %sMiB / %sMiB"}\n' "$util" "$util" "$temp" "$used" "$total"
else
    printf '{"text":"󰢮 N/A","tooltip":"GPU bulunamadı"}\n'
fi
