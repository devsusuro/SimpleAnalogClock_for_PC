# Simple Analog Clock for PC

버전 0.1.16 · Windows, macOS, Linux용 아날로그 데스크톱 시계

Windows x64 배포본은 `Simple Analog Clock for PC.exe`를 실행하면 됩니다. 이 소스에서 직접 빌드하는 방법은 아래에 있습니다.

- Windows에서는 시계 본체가 바탕화면에 떠 있는 동안 작업표시줄에 표시되지 않습니다. 시계를 종료하려면 우클릭 메뉴의 `닫기`를 선택합니다.
## 소스 빌드

Rust 1.88 이상과 대상 플랫폼의 네이티브 빌드 도구가 필요합니다. Windows에서 MSVC Rust 도구를 사용한다면 Visual Studio의 C++ 빌드 도구와 Windows SDK를 설치합니다. 프로젝트 폴더에서 다음 명령을 실행합니다.

```sh
cargo build --release --locked
```

Windows에서는 `target/release/simple-analog-clock.exe`가 생성됩니다. macOS와 Linux에서는 `target/release/simple-analog-clock`이 생성됩니다.

Clock Sans KR은 Noto Sans KR의 일부 글자를 추려 이름을 바꾼 글꼴입니다. 각 글꼴의 SIL Open Font License는 `assets` 폴더의 `OFL*.txt`에 포함되어 있습니다.
