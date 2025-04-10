# WentuZ-Backend  

[WentuZ-Backend](https://github.com/NullPointerGang/WantuZ-Backend) — это библиотека на Rust для управления аудиоплеером. Она обеспечивает воспроизведение, управление треками и потоковую обработку аудио.  

Проект разработан [FlacSy](https://github.com/FlacSy/) в качестве backend-компонента музыкального плеера [WentuZ](https://github.com/NullPointerGang/WantuZ), который разрабатывается командой [NullPointerGang](https://github.com/NullPointerGang).  

**Основные возможности:**
- Поддержка многопоточного воспроизведения  
- Интерактивное управление очередью треков  
- Поддержка популярных аудиоформатов  
- Интеграция с другими сервисами через API  
- Управление аудио устройствами

## Возможности
- Воспроизведение аудиотреков  
- Управление очередью воспроизведения (добавление, удаление, изменение порядка)  
- Перемотка, пауза, остановка и возобновление треков  
- Автоматическое воспроизведение  
- Регулировка громкости  
- Поддержка многопоточного выполнения  
- Выбор и переключение аудио устройств
- Получение списка доступных аудио устройств

## Использование

### Player

Пример использования **Player** для воспроизведения трека:

```rust
use wentuz_backend::core::player::Player;
use wentuz_backend::core::track::Track;
use std::fs;

#[tokio::main]
async fn main() {
    let player = Player::new();
    
    // Получение списка доступных аудио устройств
    if let Ok(devices) = player.list_devices().await {
        println!("Available audio devices:");
        for device in devices {
            println!("- {}", device);
        }
    }

    // Установка конкретного аудио устройства
    if let Err(e) = player.set_device("Your Audio Device").await {
        println!("Failed to set audio device: {:?}", e);
    }

    // Получение текущего устройства
    if let Some(current_device) = player.get_current_device().await {
        println!("Current audio device: {}", current_device);
    }

    let file_data = fs::read("path/to/audio.mp3").expect("Failed to read file");
    let track = Track::new("Example Track", file_data);
    player.play(track).await;
}
```

### События Player

Пример использования событий плеера **EventHandler**:

```rust
use wentuz_backend::core::player::EventHandler;
use async_trait::async_trait;

struct MyEventHandler;

#[async_trait]
impl EventHandler for MyEventHandler {
    async fn handle_event(&self, event: PlayerEvent) {
        match event {
            PlayerEvent::TrackStart(track) => {
                println!(
                    "[HANDLER] Track started: {}",
                    track.title,
                );
            }
            PlayerEvent::TrackEnd(track) => {
                println!("[HANDLER] Track ended: {}", track.title);
            }
            PlayerEvent::VolumeChanged(vol) => {
                println!("[HANDLER] Volume changed to: {:.2}", vol);
            }
            PlayerEvent::DeviceChanged(device_name) => {
                println!("[HANDLER] Audio device changed to: {}", device_name);
            }
            PlayerEvent::Error(error) => {
                println!("[HANDLER] Error occurred: {:?}", error);
            }
            _ => {}
        }
    }
}
```

Подключение **MyEventHandler**:

```rust
use core::player::Player;

#[tokio::main]
async fn main() {
    let mut player = Player::new();
    
    player.add_event_handler(Arc::new(MyEventHandler)).await;

    // Ваш код 
}
```


## Основные компоненты

### Player
**Player** - основной компонент управления воспроизведением:
- `play(track: Track)`: Запускает воспроизведение трека.
- `pause()`: Приостанавливает воспроизведение.
- `resume()`: Возобновляет воспроизведение.
- `stop()`: Останавливает текущее воспроизведение.
- `seek(position: Duration)`: Перематывает трек на указанную позицию.
- `set_volume(volume: f32)`: Устанавливает громкость.
- `auto_play()`: Включает автоматическое воспроизведение треков из очереди.
- `stop_auto_play()`: Останавливает автоматическое воспроизведение.
- `play_next()`: Воспроизводит следующий трек из очереди.
- `play_previous()`: Воспроизводит предыдущий трек.
- `list_devices() -> Result<Vec<String>, PlayerErrors>`: Возвращает список доступных аудио устройств.
- `set_device(device_name: &str) -> Result<(), PlayerErrors>`: Устанавливает аудио устройство по имени.
- `get_current_device() -> Option<String>`: Возвращает текущее аудио устройство.

### Queue
**Queue** - очередь треков:
- `add_track(track: Track)`: Добавляет трек в очередь.
- `pop_track() -> Option<Track>`: Удаляет и возвращает следующий трек из очереди.
- `peek() -> Option<&Track>`: Показывает следующий трек без удаления.
- `shuffle()`: Перемешивает треки в очереди.

### Track
**Track** - структура, содержащая информацию о треке:
- `title: String` - название трека.
- `file_data: Vec<u8>` - бинарные данные трека.
- `file_path: Option<String>` - путь к файлу (опционально).

## Баги и отчёты об ошибках

Если вы столкнулись с багом или ошибкой при использовании **WentuZ-Backend**, пожалуйста, создайте **issue** в репозитории проекта на GitHub. Это поможет нам быстро выявлять и устранять проблемы.

### Как создать баг-репорт:
1. Перейдите на страницу [WentuZ-Backend Issues](https://github.com/NullPointerGang/WantuZ-Backend/issues).
2. Нажмите на кнопку "New Issue" (Создать новый запрос).
3. Выберите шаблон для баг-репорта (если доступен) и заполните необходимые поля:
   - **Описание ошибки**: Подробно опишите, что не так, включая шаги для воспроизведения ошибки.
   - **Ожидаемое поведение**: Укажите, как система должна себя вести.
   - **Текущий результат**: Укажите, что происходит на самом деле.
   - **Окружение**: Укажите версию операционной системы, используемые зависимости (например, версия Rust), а также версию **WentuZ-Backend**.
4. Если возможно, добавьте логи ошибок или подробности об исключениях, которые возникли.

### Пример баг-репорта:
```
**Описание ошибки**:
При попытке воспроизведения трека появляется ошибка "Failed to read file".

**Ожидаемое поведение**:
Трек должен воспроизводиться без ошибок.

**Текущий результат**:
Появляется ошибка при попытке открыть файл.

**Окружение**:
- ОС: Windows 10
- Версия WentuZ-Backend: 0.1.0
- Версия Rust: 1.84.1
```

Ваши отчёты об ошибках помогут нам улучшить проект и сделать его более стабильным! Спасибо за сотрудничество!

## Лицензия
Этот проект лицензирован под [FlacSy Open Use License (FOUL) 1.0](https://github.com/FlacSy/FOUL-LICENSE/blob/main/LICENSE). Коммерческое использование возможно только с письменного разрешения автора.

## Контакты
Если у вас есть вопросы или предложения, свяжитесь с автором по email: [flacsy.tw@gmail.com](mailto:flacsy.tw@gmail.com).

