import os
import requests
import telebot
import threading
import time

from telebot.types import InlineKeyboardButton, InlineKeyboardMarkup

BOT_TOKEN = os.getenv("BOT_TOKEN")
CO2_THRESHOLD = int(os.getenv("CO2_THRESHOLD"))


if BOT_TOKEN == None:
    print("Unable to find bot token")
    exit(1)

bot = telebot.TeleBot(BOT_TOKEN)


@bot.message_handler(commands=['start', 'hello'])
def send_welcome(message):
    # bot.reply_to(message, "Welcome to the Smart Air Quality Monitoring System! How may I help you?")
    global chat_id
    chat_id = message.chat.id
    bot.send_sticker(
        chat_id, 'CAACAgUAAxkBAAEKjHFlL_ftZ4nEZXUOK80cZuvBgKOSfgACdAUAAqqlOFWjNwZ1q6p0sTAE'
    )
    buttons = [
        InlineKeyboardButton(
            text="Get Air Quality Readings",
            callback_data="Get_AirQuality"
        ),
        InlineKeyboardButton(
            text="Turn on alarm",
            callback_data="On_Alarm"
        ),
        InlineKeyboardButton(
            text="Turn off alarm",
            callback_data="Off_Alarm"
        ),

    ]
    keyboard = InlineKeyboardMarkup()
    for button in buttons:
        keyboard.add(button)
    message_text = f'Welcome to the Smart Air Quality Monitoring System! How may I help you?'
    bot.send_message(chat_id, message_text, reply_markup=keyboard)


@bot.callback_query_handler(lambda query: query.data == 'Get_AirQuality')
def handle_get_air_quality_callback(call):
    """
    Handles the execution of the respective functions upon receipt of the callback query Get_AirQuality
    """
    global chat_id
    chat_id = call.message.chat.id
    global air_quality_data
    bot.send_sticker(
        chat_id, 'CAACAgUAAxkBAAEKp2hlQgsEFHgE58pTlPAB5Jhrl2ZCTQACjwQAAuMsOFVX5zSuJAutoDME'
    )
    try:
        air_quality_response = requests.get('http://192.168.4.1:8080/jsonmetrics')

        if air_quality_response.status_code == 200:
            air_quality_data = air_quality_response.json()
            buttons = [
                InlineKeyboardButton(
                    text="Particulate Matter reading",
                    callback_data="Get_PM"
                ),
                InlineKeyboardButton(
                    text="Carbon Dioxide reading",
                    callback_data="Get_co2"
                ),
                InlineKeyboardButton(
                    text="Temperature reading",
                    callback_data="Get_temp"
                ),
                InlineKeyboardButton(
                    text="Humidity reading",
                    callback_data="Get_hum"
                ),
                InlineKeyboardButton(
                    text="Heat Index",
                    callback_data="Get_HeatIndex"
                ),
                InlineKeyboardButton(
                    text="Wifi Strength",
                    callback_data="Get_wifi"
                ),

            ]
            keyboard = InlineKeyboardMarkup()
            for button in buttons:
                keyboard.add(button)
            message_text = f'What air quality readings would you like?'
            msg = bot.send_message(chat_id, message_text,
                                   reply_markup=keyboard)
        else:
            bot.send_message(
                chat_id, "Failed to fetch air quality data. Status code: {air_quality_response.status_code}")
    except requests.exceptions.RequestException as e:
        # Handle exceptions, such as timeouts
        bot.send_message(
            chat_id, "Failed to fetch air quality data. An error occurred.")


@bot.callback_query_handler(lambda query: query.data == 'On_Alarm')
def handle_on_alarm_callback(call):
    """
    Handles the execution of the respective functions upon receipt of the callback query On_Alarm
    """
    alarm_on_response = requests.get('http://192.168.4.2/alarmon')
    global chat_id
    chat_id = call.message.chat.id
    if alarm_on_response.status_code == 200:
        bot.send_message(chat_id, "The alarm is now turned on.")
    else:
        bot.send_message(
            chat_id, "Failed to turn on the alarm. Status code: {alarm_on_response.status_code}")


@bot.callback_query_handler(lambda query: query.data == 'Off_Alarm')
def handle_off_alarm_callback(call):
    """
    Handles the execution of the respective functions upon receipt of the callback query Off_Alarm
    """
    alarm_off_response = requests.get('http://192.168.4.2/alarmoff')
    global chat_id
    chat_id = call.message.chat.id
    if alarm_off_response.status_code == 200:
        bot.send_message(chat_id, "The alarm is now turned off.")
    else:
        bot.send_message(
            chat_id, "Failed to turn off the alarm. Status code: {alarm_on_response.status_code}")


@bot.callback_query_handler(lambda query: query.data == 'Get_PM')
def handle_get_pm_callback(call):
    global chat_id
    chat_id = call.message.chat.id

    global air_quality_data  # Reuse the previously fetched data
    if air_quality_data is not None:
        pm_value = air_quality_data.get('pm25', 'N/A')
        message_text = f'Particular Matter PM2.5 value: {pm_value}'
        bot.send_message(chat_id, message_text)
    else:
        bot.send_message(
            chat_id, "Air quality data is not available. Please fetch data first.")


@bot.callback_query_handler(lambda query: query.data == 'Get_co2')
def handle_get_co2_callback(call):
    global chat_id
    chat_id = call.message.chat.id

    global air_quality_data  # Reuse the previously fetched data
    if air_quality_data is not None:
        co2_value = air_quality_data.get('co2', 'N/A')
        message_text = f'Carbon Dixoide value: {co2_value} ppm'
        bot.send_message(chat_id, message_text)
    else:
        bot.send_message(
            chat_id, "Air quality data is not available. Please fetch data first.")


@bot.callback_query_handler(lambda query: query.data == 'Get_temp')
def handle_get_temp_callback(call):
    global chat_id
    chat_id = call.message.chat.id

    global air_quality_data  # Reuse the previously fetched data
    if air_quality_data is not None:
        temp_value = air_quality_data.get('temp', 'N/A')
        message_text = f'Temperature value: {temp_value} degree Celsius'
        bot.send_message(chat_id, message_text)
    else:
        bot.send_message(
            chat_id, "Air quality data is not available. Please fetch data first.")


@bot.callback_query_handler(lambda query: query.data == 'Get_hum')
def handle_get_humidity_callback(call):
    global chat_id
    chat_id = call.message.chat.id

    global air_quality_data  # Reuse the previously fetched data
    if air_quality_data is not None:
        hum_value = air_quality_data.get('hum', 'N/A')
        message_text = f'Humidity value: {hum_value} %'
        bot.send_message(chat_id, message_text)
    else:
        bot.send_message(
            chat_id, "Air quality data is not available. Please fetch data first.")


@bot.callback_query_handler(lambda query: query.data == 'Get_HeatIndex')
def handle_get_heat_index_callback(call):
    global chat_id
    chat_id = call.message.chat.id

    global air_quality_data  # Reuse the previously fetched data
    if air_quality_data is not None:
        heatIndex_value = air_quality_data.get('heatIndex', 'N/A')
        message_text = f'Heat Index value: {heatIndex_value} degree Celsius'
        bot.send_message(chat_id, message_text)
    else:
        bot.send_message(
            chat_id, "Air quality data is not available. Please fetch data first.")


@bot.callback_query_handler(lambda query: query.data == 'Get_wifi')
def handle_get_wifi_strength_callback(call):
    global chat_id
    chat_id = call.message.chat.id

    global air_quality_data  # Reuse the previously fetched data
    if air_quality_data is not None:
        wifi_strength = air_quality_data.get('wifi', 'N/A')
        message_text = f'Wi-Fi Strength: {wifi_strength} dBm'
        bot.send_message(chat_id, message_text)
    else:
        bot.send_message(
            chat_id, "Air quality data is not available. Please fetch data first.")


def query_data():
    try:
        wifi_strength = air_quality_data.get('co2', 'N/A')
        return wifi_strength
    except NameError:
        return None


def check_condition(data):
    return data > 1500


def get_chat_id():
    global chat_id
    try:
        return chat_id
    except NameError:
        return None


def send_alert(chat_id):
    bot.send_message(chat_id, "Carbon dioxide levels is at dangerous levels!")


def data_check_loop():
    was_dangerous = False
    while True:
        data = query_data()
        chat_id = get_chat_id()
        if data != None and check_condition(data) and chat_id != None and not was_dangerous:
            send_alert(chat_id)
            was_dangerous = True
        if data != None and check_condition(data) and was_dangerous:
            was_dangerous = False
        time.sleep(10)


def start_data_check_thread():
    thread = threading.Thread(target=data_check_loop)
    thread.start()


start_data_check_thread()
bot.infinity_polling()
