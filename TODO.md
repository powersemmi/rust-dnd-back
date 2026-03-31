# DnD-VTT Tasks TODO

## Bugs

## Security

### Backend

### Infrastructure

* Поставить Cloudflare перед сервером
  * Скрывает реальный IP от атакующих
  * Бесплатный тариф закрывает объёмные DDoS атаки
  * Включить "Under Attack Mode" при необходимости

* Ограничить количество одновременных WS соединений с одного IP
  * Настроить в nginx/caddy: не более 10 соединений с одного IP

* Если Redis станет узким местом из-за файлового трафика - мигрировать FileChunk на WebRTC DataChannel:
  - Сигналинг (SDP offer/answer, ICE кандидаты) остаётся через существующий WS
  - Сами чанки летят напрямую между браузерами, backend не видит файловый трафик
  - Остальной протокол (FileAnnounce, FileRequest, FileAbort) не меняется
  - Для NAT traversal понадобится STUN сервер (бесплатный Google: stun.l.google.com:19302)
  - При симметричном NAT потребуется TURN сервер (платный, coturn self-hosted)

## New Features

### Frontend
