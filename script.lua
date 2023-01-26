---@diagnostic disable: lowercase-global
local wifi = require "wifi"

function init()
  wifi.setcountry({country="FR", start_ch=1, end_ch=13, policy=wifi.COUNTRY_AUTO})
  wifi.setmode(wifi.STATION)
end

function start_sniff()
  wifi.monitor.start(13, 0x40, function(pkt)
    print(pkt.dstmac_hex, pkt.rssi, pkt.ie_ssid)
    end)
end

function sniff()
  
end

function process()
  
end

function send()
  
end

init()
start_sniff()
