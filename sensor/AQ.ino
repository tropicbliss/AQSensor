/*
Important: This code is only for the DIY PRO PCB Version 3.7 that has a push button mounted.
This is the code for the AirGradient DIY PRO Air Quality Sensor with an ESP8266 Microcontroller.
It is a high quality sensor showing PM2.5, CO2, Temperature and Humidity on a small display and can send data over Wifi.
Build Instructions: https://www.airgradient.com/open-airgradient/instructions/diy-pro-v37/
Kits (including a pre-soldered version) are available: https://www.airgradient.com/open-airgradient/kits/
The codes needs the following libraries installed:
“WifiManager by tzapu, tablatronix” tested with version 2.0.11-beta
“U8g2” by oliver tested with version 2.32.15
Configuration:
Please set in the code below the configuration parameters.
If you have any questions please visit our forum at https://forum.airgradient.com/
If you are a school or university contact us for a free trial on the AirGradient platform.
https://www.airgradient.com/
CC BY-SA 4.0 Attribution-ShareAlike 4.0 International License
Code modified by tropicbliss
*/


#include <AirGradient.h>
#include <WiFiManager.h>
#include <ESP8266WiFi.h>
#include <ESP8266HTTPClient.h>
#include <WiFiClient.h>
#include <EEPROM.h>
#include <U8g2lib.h>

AirGradient ag = AirGradient();

// for peristent saving and loading
int addr = 4;

// Display bottom right
U8G2_SH1106_128X64_NONAME_F_HW_I2C u8g2(U8G2_R0, /* reset=*/ U8X8_PIN_NONE);

// CONFIGURATION START

//set to the endpoint you would like to use
String API = "";

// set to true to switch from Celcius to Fahrenheit
boolean inF = false;

// PM2.5 in US AQI (default ug/m3)
boolean inUSAQI = false;

// Display Position
boolean displayTop = true;

// set to true if you want to connect to wifi. You have 60 seconds to connect. Then it will go into an offline mode.
boolean connectWIFI=true;

// CONFIGURATION END


unsigned long currentMillis = 0;

const int sendToServerInterval = 10000;
unsigned long previoussendToServer = 0;

const int co2Interval = 5000;
unsigned long previousCo2 = 0;
int Co2 = 0;

const int pm25Interval = 5000;
unsigned long previousPm25 = 0;
int pm25 = 0;

const int tempHumInterval = 2500;
unsigned long previousTempHum = 0;
float temp = 0;
int hum = 0;

int buttonConfig=4;
int lastState = LOW;
int currentState;
unsigned long pressedTime  = 0;
unsigned long releasedTime = 0;

void setup() {
  Serial.begin(115200);
  Serial.println("Hello");
  u8g2.begin();

  EEPROM.begin(512);
  delay(500);

  buttonConfig = String(EEPROM.read(addr)).toInt();
  setConfig();

  updateOLED2("Press Button", "Now for", "Config Menu");
  delay(2000);

  currentState = digitalRead(D7);
  if (currentState == HIGH)
  {
    updateOLED2("Entering", "Config Menu", "");
    delay(3000);
    lastState = LOW;
    inConf();
  }

  if (connectWIFI)
  {
     connectToWifi();
  }

  updateOLED2("Warming Up", "Serial Number:", String(ESP.getChipId(), HEX));
  ag.CO2_Init();
  ag.PMS_Init();
  ag.TMP_RH_Init(0x44);
  updateOLED2("Setup Success!", "Turning Off", "Display...");
  delay(6000);
  u8g2.clear();
}

void loop() {
  currentMillis = millis();
  updateCo2();
  updatePm25();
  updateTempHum();
  sendToServer();
}

void inConf(){
  setConfig();
  currentState = digitalRead(D7);

  if(lastState == LOW && currentState == HIGH) {
    pressedTime = millis();
  }

  else if(lastState == HIGH && currentState == LOW) {
    releasedTime = millis();
    long pressDuration = releasedTime - pressedTime;
    if( pressDuration < 1000 ) {
      buttonConfig=buttonConfig+1;
      if (buttonConfig>7) buttonConfig=0;
    }
  }

  if (lastState == HIGH && currentState == HIGH){
     long passedDuration = millis() - pressedTime;
      if( passedDuration > 4000 ) {
          updateOLED2("Saved", "Release", "Button Now");
          delay(1000);
          updateOLED2("Rebooting", "in", "5 seconds");
          delay(5000);
          EEPROM.write(addr, char(buttonConfig));
          EEPROM.commit();
          delay(1000);
          ESP.restart();
    }

  }
  lastState = currentState;
  delay(100);
  inConf();
}

void setConfig() {
  if (buttonConfig == 0) {
    updateOLED2("Temp. in C", "PM in ug/m3", "Display Top");
      u8g2.setDisplayRotation(U8G2_R2);
      inF = false;
      inUSAQI = false;
  } else if (buttonConfig == 1) {
    updateOLED2("Temp. in C", "PM in US AQI", "Display Top");
      u8g2.setDisplayRotation(U8G2_R2);
      inF = false;
      inUSAQI = true;
  } else if (buttonConfig == 2) {
    updateOLED2("Temp. in F", "PM in ug/m3", "Display Top");
      u8g2.setDisplayRotation(U8G2_R2);
      inF = true;
      inUSAQI = false;
  } else if (buttonConfig == 3) {
    updateOLED2("Temp. in F", "PM in US AQI", "Display Top");
      u8g2.setDisplayRotation(U8G2_R2);
       inF = true;
      inUSAQI = true;
  } else if (buttonConfig == 4) {
    updateOLED2("Temp. in C", "PM in ug/m3", "Display Bottom");
      u8g2.setDisplayRotation(U8G2_R0);
      inF = false;
      inUSAQI = false;
  }
    if (buttonConfig == 5) {
    updateOLED2("Temp. in C", "PM in US AQI", "Display Bottom");
      u8g2.setDisplayRotation(U8G2_R0);
      inF = false;
      inUSAQI = true;
  } else if (buttonConfig == 6) {
    updateOLED2("Temp. in F", "PM in ug/m3", "Display Bottom");
    u8g2.setDisplayRotation(U8G2_R0);
      inF = true;
      inUSAQI = false;
  } else  if (buttonConfig == 7) {
    updateOLED2("Temp. in F", "PM in US AQI", "Display Bottom");
      u8g2.setDisplayRotation(U8G2_R0);
       inF = true;
      inUSAQI = true;
  }
}

void updateCo2()
{
    if (currentMillis - previousCo2 >= co2Interval) {
      previousCo2 += co2Interval;
      Co2 = ag.getCO2_Raw();
      Serial.println(String(Co2));
    }
}

void updatePm25()
{
    if (currentMillis - previousPm25 >= pm25Interval) {
      previousPm25 += pm25Interval;
      pm25 = ag.getPM2_Raw();
      Serial.println(String(pm25));
    }
}

void updateTempHum()
{
    if (currentMillis - previousTempHum >= tempHumInterval) {
      previousTempHum += tempHumInterval;
      TMP_RH result = ag.periodicFetchData();
      temp = result.t;
      hum = result.rh;
      Serial.println(String(temp));
    }
}

void updateOLED2(String ln1, String ln2, String ln3) {
  char buf[9];
  u8g2.firstPage();
  u8g2.firstPage();
  do {
  u8g2.setFont(u8g2_font_t0_16_tf);
  u8g2.drawStr(1, 10, String(ln1).c_str());
  u8g2.drawStr(1, 30, String(ln2).c_str());
  u8g2.drawStr(1, 50, String(ln3).c_str());
    } while ( u8g2.nextPage() );
}

void sendToServer() {
   if (currentMillis - previoussendToServer >= sendToServerInterval) {
     previoussendToServer += sendToServerInterval;
      String payload = "{\"wifi\":" + String(WiFi.RSSI())
      + (Co2 < 0 ? "" : ", \"rco2\":" + String(Co2))
      + (pm25 < 0 ? "" : ", \"pm02\":" + String(pm25))
      + ", \"atmp\":" + String(temp)
      + (hum < 0 ? "" : ", \"rhum\":" + String(hum))
      + "}";

      if(WiFi.status()== WL_CONNECTED){
        Serial.println(payload);
        Serial.println(API);
        WiFiClient client;
        HTTPClient http;
        http.begin(client, API);
        http.addHeader("content-type", "application/json");
        int httpCode = http.POST(payload);
        String response = http.getString();
        Serial.println(httpCode);
        Serial.println(response);
        http.end();
      }
      else {
        Serial.println("WiFi Disconnected");
      }
   }
}

// Wifi Manager
 void connectToWifi() {
   WiFiManager wifiManager;
   //WiFi.disconnect(); //to delete previous saved hotspot
   String HOTSPOT = "AG-" + String(ESP.getChipId(), HEX);
   updateOLED2("90s to connect", "to Wifi Hotspot", HOTSPOT);
   wifiManager.setTimeout(90);

   if (!wifiManager.autoConnect((const char * ) HOTSPOT.c_str())) {
     updateOLED2("booting into", "offline mode", "");
     Serial.println("failed to connect and hit timeout");
     delay(6000);
   }
}