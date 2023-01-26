---@diagnostic disable: lowercase-global
local wifi = require "wifi"
local tmr = require "tmr"

-- Canal initial
CHANNEL = 1

function init()
  wifi.setcountry({country="FR", start_ch=1, end_ch=13, policy=wifi.COUNTRY_AUTO})
  -- Passage en mode station
  wifi.setmode(wifi.STATION)
end

function start_sniff()
  -- Activer le moniteur
  -- 13 : Premier bit de la trame
  -- 0x40 : Filtrer que pour les probe requests
  -- if #pkt : Recupere la longueur du SSID et si elle est nulle, ne pas l'inclure
  -- \2 : ASCII Debut texte
  -- \31 : ASCII Separation d'unite
  -- \3 : ASCII Fin de transmission
  wifi.monitor.start(13, 0x40, function(pkt)
    if ( #pkt.ie_ssid == 0 ) then
        send = string.format("\2%d\31%s\31%d\31\3", pkt.channel, pkt.dstmac_hex, pkt.rssi)
    else
        send = string.format("\2%d\31%s\31%d\31%s\31\3", pkt.channel, pkt.dstmac_hex, pkt.rssi, pkt.ie_ssid)
    end
    print(send)
    end)
end

function sniff()
-- Iterer entre les canaux
  if ( CHANNEL < 13 ) then
    CHANNEL = CHANNEL + 1
  elseif ( CHANNEL == 13 ) then
    CHANNEL = 1
  end
  wifi.monitor.channel(CHANNEL)
end

-- Creer un timer pour le changement de channel
channel_switcher = tmr.create()
channel_switcher:register(5000, tmr.ALARM_AUTO, function() sniff() end)

init()
start_sniff()
channel_switcher:start()
