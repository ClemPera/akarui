# akarui

## how to make it work
create a `cfg.toml` file and complete the infos inside (this is your env file):
```toml
[akarui]
wifi_ssid = ""
wifi_pass = ""
```

# Dev stuff
## doc
[official yeelight communication pdf doc](https://www.yeelight.com/download/Yeelight_Inter-Operation_Spec.pdf)

## discovering
```bash
echo -e 'M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1982\r\nMAN: "ssdp:discover"\r\nST: wifi_bulb\r\n\r\n' | socat - UDP4-DATAGRAM:239.255.255.250:1982,broadcast
```

## sending
```bash
printf '{"id":1,"method":"toggle","params":[]}\r\n' | nc 192.168.1.171 55443
```

## listen to advertise (didn't try it)
```bash
nc -u -l 1982
``` 