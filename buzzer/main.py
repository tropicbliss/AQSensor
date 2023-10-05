# CC BY-SA 4.0 Attribution-ShareAlike 4.0 International License
# Code written by tropicbliss

from picozero import Speaker, pico_temp_sensor
from time import sleep
import network
import socket
import machine
import uasyncio as asyncio
from uasyncio import Event
import time
import secrets

SSID = secrets.SSID
PASSWORD = secrets.PASSWORD

speaker = Speaker(20)
play = False
event = Event()

def webpage():
    global play
    state = "ON" if play else "OFF"
    temperature = pico_temp_sensor.temp
    return f"""<!DOCTYPE html><html><form action="/alarmon"><input type="submit" value="Alarm on"></form><form action="/alarmoff"><input type="submit" value="Alarm off"></form><p>Alarm is {state}</p><p>Temperature is {temperature}</p></body></html>"""

async def beep():
    global play, event
    while True:
        if play:
            speaker.play(262, 0.1, wait=False)
            await asyncio.sleep(0.2)
            speaker.play(262, 0.6, wait=False)
            await asyncio.sleep(1.6)
        else:
            await event.wait()

def connect():
    wlan = network.WLAN(network.STA_IF)
    wlan.active(True)
    wlan.config(pm = 0xa11140)
    wlan.ifconfig(('192.168.4.2', '255.255.255.0', '0.0.0.0', '0.0.0.0'))
    wlan.connect(SSID, PASSWORD)
    max_wait = 10
    while max_wait > 0:
        if wlan.status() < 0 or wlan.status() >= 3:
            break
        max_wait -= 1
        print("Waiting for connection...")
        time.sleep(1)
    if wlan.status() != 3:
        raise RuntimeError('network connection failed')
    else:
        ip = wlan.ifconfig()[0]
        print(f"Connected on {ip}")

async def serve(reader, writer):
    global play, event
    request_line = await reader.readline()
    while await reader.readline() != b"\r\n":
        pass
    request = str(request_line)
    if request.find("/alarmon") == 6:
        play = True
        event.set()
    elif request.find("/alarmoff") == 6:
        play = False
        event.clear()
    response = webpage()
    content_length = len(response)
    writer.write("HTTP/1.1 200 OK\r\nContent-Length: ")
    writer.write(str(content_length))
    writer.write("\r\nContent-type: text/html\r\nConnection: close\r\n\r\n")
    writer.write(response)
    await writer.drain()
    await writer.wait_closed()

async def main():
    connect()
    asyncio.create_task(asyncio.start_server(serve, "0.0.0.0", 80))
    await beep()

try:
    asyncio.run(main())
except Exception:
    machine.reset()
