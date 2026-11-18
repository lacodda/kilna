//! A demo corpus for a space that wants one (v0.129, FR-16): a handful of
//! works at every stage, versions to compare, a score, notes, a card of the
//! canon and releases on the calendar of the coming days - so a new person
//! sees each screen with something on it.
//!
//! Made up, neutral and in English: no real person, no real title, nothing of
//! anyone's own (the privacy rule of the line). Written through the same
//! gestures a person makes, so every bit of it is in the history and can be
//! undone or deleted like anything else; the dates are counted from today, so
//! the calendar is never a page of the past.

use rusqlite::Connection;
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::actions;
use crate::error::{Error, Result};
use crate::note::NewNote;
use crate::profile::config::ProfileConfig;
use crate::score::NewScore;
use crate::work::version::NewVersion;
use crate::work::{NewWork, WorkPatch};

/// What a demo put into a space.
#[derive(Debug, Clone, Default, Serialize, ts_rs::TS)]
pub struct DemoReport {
    pub works: usize,
    pub notes: usize,
    pub releases: usize,
}

/// One made-up work: its title, how far along it is, and its text - in
/// versions, oldest first, when it has any.
struct Piece {
    title: &'static str,
    stage: i64,
    versions: &'static [&'static str],
}

/// The words of one craft's demo.
struct Corpus {
    pieces: &'static [Piece],
    /// The text of the brief, for the piece still at its idea.
    brief: &'static str,
    notes: &'static [(&'static str, &'static str)],
    /// A card of the canon: its kind, name and what is known of it.
    card: Option<(&'static str, &'static str, &'static str)>,
}

const STUDIO: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Paper Lanterns",
            stage: 100,
            versions: &[
                "We hung paper lanterns on the fence\nand the wind took two of them by ten\n\nCount the ones that stayed\ncount the ones that stayed",
                "We hung paper lanterns on the fence\nthe wind took two before the rain came in\n\nCount the ones that stayed lit\ncount the ones that stayed",
            ],
        },
        Piece {
            title: "Low Tide Hours",
            stage: 75,
            versions: &[
                "The harbour shows its ribs at noon\nboats lean on mud like tired men\nI keep the hours the water keeps\nand wait for it to come again",
            ],
        },
        Piece {
            title: "Glass Orchard",
            stage: 50,
            versions: &[
                "Apples of glass on a wire tree\nring when the bus goes past\n(second verse - something about the frost)",
            ],
        },
        Piece {
            title: "Northbound",
            stage: 25,
            versions: &[],
        },
        Piece {
            title: "A song about the lift",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## What it is about\nA night train going north, and someone who will not say why they are on it.\n\n## Mood\nQuiet, steady, a little cold.",
    notes: &[
        ("phrase", "the ones that stayed lit"),
        ("phrase", "boats lean on mud like tired men"),
        (
            "note",
            "A song in which nothing happens but the light changes. Maybe the lift song.",
        ),
    ],
    card: Some((
        "channel",
        "The demo channel",
        "A small made-up studio that writes quiet songs about ordinary places: harbours, orchards, night trains.",
    )),
};

const NOVEL: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Chapter 1. The Keeper's Daughter",
            stage: 100,
            versions: &[
                "The lamp at Gull Point had not gone out in forty years. Mira checked it anyway, every night, the way her father had.",
                "The lamp at Gull Point had not gone out in forty years. Mira climbed the ninety steps anyway, every night, the way her father had, counting them under her breath.",
            ],
        },
        Piece {
            title: "Chapter 2. The Letter",
            stage: 50,
            versions: &[
                "The letter came on a Tuesday, which was wrong: the boat only came on Fridays.",
            ],
        },
        Piece {
            title: "Chapter 3",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## What happens\nMira finds out who sent the letter.\n\n## What it changes\nShe stops counting the steps.",
    notes: &[
        ("phrase", "counting them under her breath"),
        (
            "note",
            "Does the boat ever come on time? Decide before chapter 4.",
        ),
    ],
    card: Some((
        "character",
        "Mira",
        "Keeps the lighthouse at Gull Point after her father. Counts things when she is afraid.",
    )),
};

const PODCAST: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Episode 1. Starting From Silence",
            stage: 100,
            versions: &[
                "Cold open: the sound of an empty room. HOST: This is what our studio sounded like before the first episode. Today: why we almost didn't start.",
            ],
        },
        Piece {
            title: "Episode 2. The Guest Who Never Came",
            stage: 50,
            versions: &[
                "HOST: We had a guest booked. We had questions. We had coffee. Here is what happened instead.",
            ],
        },
        Piece {
            title: "Episode 3",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## The question\nWhat do you do with the hour a guest leaves empty?\n\n## The way in\nStart with the coffee going cold.",
    notes: &[
        ("phrase", "here is what happened instead"),
        (
            "note",
            "A segment of listener questions, once there are listeners.",
        ),
    ],
    card: Some((
        "guest",
        "A guest to invite",
        "A made-up guest: a lighthouse keeper who records the sea every night.",
    )),
};

const BLOG: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Why Small Tools Win",
            stage: 100,
            versions: &[
                "Small tools do one thing. That is the whole argument, and it is enough.",
                "Small tools do one thing, and you can see all of it. That is the whole argument, and it turns out to be enough.",
            ],
        },
        Piece {
            title: "A Week Without Notifications",
            stage: 50,
            versions: &[
                "Day one: I turned everything off. Day two: I turned one thing back on, and here is why.",
            ],
        },
        Piece {
            title: "Notes on Writing Every Day",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## The point\nWriting every day matters less than writing on the bad days.\n\n## For whom\nAnyone who stopped after a week.",
    notes: &[
        ("phrase", "you can see all of it"),
        (
            "note",
            "A post on what to cut from a draft, with a real draft cut on the page.",
        ),
    ],
    card: None,
};

const POETRY: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Inventory of a Kitchen at Night",
            stage: 100,
            versions: &[
                "the kettle cooling\nthe knife asleep on the board\none cup turned down",
                "the kettle cooling\nthe knife asleep on its board\none cup face down, waiting",
            ],
        },
        Piece {
            title: "Winter Ferry",
            stage: 50,
            versions: &[
                "The ferry leaves at six and no one waves.\nThe water folds the town back into grey.",
            ],
        },
        Piece {
            title: "A poem about the stairwell",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## The image it starts from\nA light left on in an empty stairwell.\n\n## What it turns on\nWho it was left on for.",
    notes: &[
        ("phrase", "one cup face down, waiting"),
        ("note", "Try the ferry poem in couplets."),
    ],
    card: None,
};

const NEWSLETTER: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Issue 1. Why I Started This",
            stage: 100,
            versions: &[
                "Hello. This is a letter about small tools, sent once a week.",
                "Hello. This is a weekly letter about small tools - the ones you can see all of.",
            ],
        },
        Piece {
            title: "Issue 2. Three Tools I Stopped Using",
            stage: 50,
            versions: &[
                "This week: three tools I dropped, and the one thing each of them did better than its replacement.",
            ],
        },
        Piece {
            title: "Issue 3",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## What this issue is about\nThe tool I use every day and never mention.\n\n## The one thing to take away\nBoring tools are the ones that last.",
    notes: &[
        ("phrase", "the ones you can see all of"),
        ("note", "A reader asked about backups - an issue on that."),
    ],
    card: None,
};

// The same demos in Russian, the window's second language (v0.130): what a
// person who reads the window in Russian is given, in place of the English.
const STUDIO_RU: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Бумажные фонари",
            stage: 100,
            versions: &[
                "Мы повесили фонари на забор\nи два из них ветер унёс к десяти\n\nСчитай те, что остались\nсчитай те, что остались",
                "Мы повесили фонари на забор\nдва унесло, пока не пошёл дождь\n\nСчитай те, что горят\nсчитай те, что остались",
            ],
        },
        Piece {
            title: "Часы отлива",
            stage: 75,
            versions: &[
                "В полдень гавань показывает рёбра\nлодки легли на ил, как усталые люди\nя держу часы, что держит вода\nи жду, когда она вернётся",
            ],
        },
        Piece {
            title: "Стеклянный сад",
            stage: 50,
            versions: &[
                "Стеклянные яблоки на проволочном дереве\nзвенят, когда проходит автобус\n(второй куплет - что-то про иней)",
            ],
        },
        Piece {
            title: "На север",
            stage: 25,
            versions: &[],
        },
        Piece {
            title: "Песня про лифт",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## О чём это\nНочной поезд на север и человек, который не скажет, зачем он в нём.\n\n## Настроение\nТихое, ровное, чуть холодное.",
    notes: &[
        ("phrase", "те, что горят"),
        ("phrase", "лодки легли на ил, как усталые люди"),
        (
            "note",
            "Песня, в которой ничего не происходит, только меняется свет. Может, это песня про лифт.",
        ),
    ],
    card: Some((
        "channel",
        "Демо-канал",
        "Небольшая выдуманная студия, которая пишет тихие песни про обычные места: гавани, сады, ночные поезда.",
    )),
};

const NOVEL_RU: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Глава 1. Дочь смотрителя",
            stage: 100,
            versions: &[
                "Маяк на Чаячьем мысу не гас сорок лет. Мира всё равно проверяла его каждую ночь, как отец.",
                "Маяк на Чаячьем мысу не гас сорок лет. Мира всё равно поднималась на девяносто ступеней каждую ночь, как отец, считая их вполголоса.",
            ],
        },
        Piece {
            title: "Глава 2. Письмо",
            stage: 50,
            versions: &[
                "Письмо пришло во вторник, и это было неправильно: катер приходил только по пятницам.",
            ],
        },
        Piece {
            title: "Глава 3",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## Что происходит\nМира узнаёт, кто прислал письмо.\n\n## Что это меняет\nОна перестаёт считать ступени.",
    notes: &[
        ("phrase", "считая их вполголоса"),
        (
            "note",
            "Приходит ли катер хоть раз вовремя? Решить до четвёртой главы.",
        ),
    ],
    card: Some((
        "character",
        "Мира",
        "Смотрит за маяком на Чаячьем мысу после отца. Считает, когда ей страшно.",
    )),
};

const PODCAST_RU: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Эпизод 1. Начать с тишины",
            stage: 100,
            versions: &[
                "Холодное начало: звук пустой комнаты. ВЕДУЩИЙ: Так звучала наша студия до первого эпизода. Сегодня - почему мы чуть не начали.",
            ],
        },
        Piece {
            title: "Эпизод 2. Гость, который не пришёл",
            stage: 50,
            versions: &[
                "ВЕДУЩИЙ: У нас был гость. У нас были вопросы. У нас был кофе. Вот что вышло вместо этого.",
            ],
        },
        Piece {
            title: "Эпизод 3",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## Вопрос\nЧто делать с часом, который гость оставил пустым?\n\n## Вход\nНачать с остывающего кофе.",
    notes: &[
        ("phrase", "вот что вышло вместо этого"),
        (
            "note",
            "Рубрика вопросов слушателей - когда появятся слушатели.",
        ),
    ],
    card: Some((
        "guest",
        "Гость, которого стоит позвать",
        "Выдуманный гость: смотритель маяка, который каждую ночь записывает море.",
    )),
};

const BLOG_RU: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Почему маленькие инструменты выигрывают",
            stage: 100,
            versions: &[
                "Маленький инструмент делает одно дело. Вот и весь довод, и его хватает.",
                "Маленький инструмент делает одно дело, и его видно целиком. Вот и весь довод, и его, оказывается, хватает.",
            ],
        },
        Piece {
            title: "Неделя без уведомлений",
            stage: 50,
            versions: &[
                "День первый: выключил всё. День второй: включил одно обратно, и вот почему.",
            ],
        },
        Piece {
            title: "Заметки о том, чтобы писать каждый день",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## Мысль\nПисать каждый день важно меньше, чем писать в плохие дни.\n\n## Для кого\nДля всех, кто бросил через неделю.",
    notes: &[
        ("phrase", "его видно целиком"),
        (
            "note",
            "Пост о том, что вырезать из черновика, - с настоящим черновиком, вырезанным на странице.",
        ),
    ],
    card: None,
};

const POETRY_RU: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Опись ночной кухни",
            stage: 100,
            versions: &[
                "чайник остывает\nнож уснул на доске\nчашка вверх дном",
                "чайник остывает\nнож уснул на своей доске\nчашка вверх дном и ждёт",
            ],
        },
        Piece {
            title: "Зимний паром",
            stage: 50,
            versions: &[
                "Паром уходит в шесть, и никто не машет.\nВода складывает город обратно в серое.",
            ],
        },
        Piece {
            title: "Стихотворение про лестницу",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## Образ, с которого начинается\nСвет, оставленный в пустом подъезде.\n\n## На чём поворот\nДля кого его оставили.",
    notes: &[
        ("phrase", "чашка вверх дном и ждёт"),
        ("note", "Попробовать паром двустишиями."),
    ],
    card: None,
};

const NEWSLETTER_RU: Corpus = Corpus {
    pieces: &[
        Piece {
            title: "Выпуск 1. Зачем я это начал",
            stage: 100,
            versions: &[
                "Привет. Это письмо о маленьких инструментах, раз в неделю.",
                "Привет. Это еженедельное письмо о маленьких инструментах - тех, что видно целиком.",
            ],
        },
        Piece {
            title: "Выпуск 2. Три инструмента, которые я бросил",
            stage: 50,
            versions: &[
                "На этой неделе: три инструмента, от которых я отказался, и то, что каждый делал лучше замены.",
            ],
        },
        Piece {
            title: "Выпуск 3",
            stage: 0,
            versions: &[],
        },
    ],
    brief: "## О чём этот выпуск\nИнструмент, которым я пользуюсь каждый день и никогда не упоминаю.\n\n## Что унести\nСкучные инструменты живут дольше всех.",
    notes: &[
        ("phrase", "те, что видно целиком"),
        (
            "note",
            "Читатель спросил про резервные копии - выпуск об этом.",
        ),
    ],
    card: None,
};

/// Put the demo of the space's craft into it. Refused for a space that
/// already holds works: a demo is a way to look around a new space, never
/// something mixed into a person's own.
pub fn seed(conn: &Connection) -> Result<DemoReport> {
    seed_in(conn, "en")
}

/// The demo in a language: Russian for the window's second language, English
/// - the product's - for any other.
pub fn seed_in(conn: &Connection, language: &str) -> Result<DemoReport> {
    let works: i64 = conn.query_row("SELECT count(*) FROM work", [], |row| row.get(0))?;
    if works > 0 {
        return Err(Error::refused("demo.notEmpty"));
    }
    let profile =
        crate::profile::active(conn)?.ok_or_else(|| Error::refused("profile.noneActive"))?;
    let russian = language.starts_with("ru");
    let corpus = match (profile.key.as_str(), russian) {
        ("music", false) => &STUDIO,
        ("music", true) => &STUDIO_RU,
        ("novel", false) => &NOVEL,
        ("novel", true) => &NOVEL_RU,
        ("podcast", false) => &PODCAST,
        ("podcast", true) => &PODCAST_RU,
        ("poetry", false) => &POETRY,
        ("poetry", true) => &POETRY_RU,
        ("newsletter", false) => &NEWSLETTER,
        ("newsletter", true) => &NEWSLETTER_RU,
        (_, false) => &BLOG,
        (_, true) => &BLOG_RU,
    };
    let config = &profile.config;
    let kind = first_writing_kind(config).ok_or_else(|| Error::refused("demo.noKind"))?;
    let mut report = DemoReport::default();
    let mut finished = None;

    for piece in corpus.pieces {
        let work = actions::work::create(
            conn,
            NewWork {
                kind: kind.clone(),
                title: piece.title.to_owned(),
                ..NewWork::default()
            },
        )?;
        report.works += 1;
        let role = config
            .vocabulary(&kind)
            .version_roles
            .first()
            .map(|r| r.key.clone());
        if let Some(role) = &role {
            for (index, body) in piece.versions.iter().enumerate() {
                actions::version::create(
                    conn,
                    &work.id,
                    NewVersion {
                        role: role.clone(),
                        body: (*body).to_owned(),
                        label: None,
                        meta: None,
                        make_current: index + 1 == piece.versions.len(),
                        parent_version_id: None,
                        trial_id: None,
                    },
                )?;
            }
        }
        // The piece still at its idea carries its brief, where the kind
        // prepares a work on a conception.
        if piece.versions.is_empty()
            && piece.stage > 0
            && let Some(conception) = config.conception(&kind)
        {
            actions::version::create(
                conn,
                &work.id,
                NewVersion {
                    role: conception.brief.clone(),
                    body: corpus.brief.to_owned(),
                    label: None,
                    meta: None,
                    make_current: true,
                    parent_version_id: None,
                    trial_id: None,
                },
            )?;
        }
        actions::work::update(
            conn,
            &work.id,
            WorkPatch {
                stage: Some(Some(piece.stage)),
                ..WorkPatch::default()
            },
        )?;
        if piece.stage == 100 && !piece.versions.is_empty() {
            score(conn, config, &kind, &work.id)?;
            finished = Some(work.id.clone());
        }
    }

    for (note_kind, body) in corpus.notes {
        let known = config.note_kind(note_kind).is_some();
        actions::note::create(
            conn,
            NewNote {
                body: (*body).to_owned(),
                kind: known.then(|| (*note_kind).to_owned()),
                ..NewNote::default()
            },
        )?;
        report.notes += 1;
    }
    if let Some((card_kind, name, body)) = corpus.card
        && config.card_kind(card_kind).is_some()
    {
        actions::canon::create_card(
            conn,
            NewNote {
                body: body.to_owned(),
                kind: Some(card_kind.to_owned()),
                title: Some(name.to_owned()),
                ..NewNote::default()
            },
        )?;
        report.notes += 1;
    }

    // What the finished piece goes out as, booked for the coming days: the
    // piece itself where its kind goes out, as a post or a poem does; what is
    // made from it where it does not, as a song goes out as its clip.
    let doors: Vec<String> = config
        .vocabulary(&kind)
        .release_kinds
        .iter()
        .map(|door| door.key.clone())
        // A publication goes out once: through its first door.
        .take(1)
        .collect();
    if let Some(source) = finished.as_ref().filter(|_| !doors.is_empty()) {
        for (offset, door) in doors.iter().enumerate() {
            actions::release::create(
                conn,
                crate::release::NewRelease {
                    work_id: source.clone(),
                    kind: door.clone(),
                    scheduled_at: Some(days_from_today(3 + 4 * offset as i64)),
                    meta: None,
                    scheduled_time: None,
                    time_zone: None,
                },
            )?;
            report.releases += 1;
        }
    } else if let Some(source) = finished {
        let made_of: Vec<String> = config
            .work_kinds
            .iter()
            .filter(|k| k.key != kind && k.has_doors() && k.lab.is_none())
            .map(|k| k.key.clone())
            .take(2)
            .collect();
        for (offset, made_kind) in made_of.iter().enumerate() {
            let locale = if russian { "ru" } else { "en" };
            let made = actions::work::derive(conn, &source, made_kind, None, Some(locale), None)?;
            report.works += 1;
            if let Some(release) = made.release_id {
                let day = days_from_today(3 + 4 * offset as i64);
                actions::release::schedule(conn, &release, &day)?;
                report.releases += 1;
            }
        }
    }
    Ok(report)
}

/// The file that marks an application folder as a demo stand: one kilna
/// made with `--demo-all`, and may wipe and make again.
pub const STAND_MARKER: &str = "demo-stand";

/// A demo stand (v0.130, FR-16): an application folder holding one space of
/// every shipped craft, each filled with its demo - made from nothing, or
/// wiped and made again. Refused for a folder that holds spaces and was not
/// made a stand: nothing a person wrote is ever wiped by a flag.
pub fn stand(app_dir: &std::path::Path) -> Result<Vec<(String, DemoReport)>> {
    let marker = app_dir.join(STAND_MARKER);
    let spaces = app_dir.join(crate::space::SPACES_DIR);
    let database = app_dir.join(crate::db::APP_DATABASE_FILE);
    let holds = spaces.is_dir()
        && std::fs::read_dir(&spaces)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false);
    if holds && !marker.is_file() {
        return Err(Error::refused("demo.notAStand").param("path", app_dir.display().to_string()));
    }
    if spaces.is_dir() {
        std::fs::remove_dir_all(&spaces).map_err(|cause| {
            Error::refused("file.notWritten")
                .param("path", spaces.display().to_string())
                .param("cause", cause.to_string())
        })?;
    }
    for file in ["", "-wal", "-shm"] {
        let path = std::path::PathBuf::from(format!("{}{file}", database.display()));
        if path.is_file() {
            std::fs::remove_file(&path).map_err(|cause| {
                Error::refused("file.notWritten")
                    .param("path", path.display().to_string())
                    .param("cause", cause.to_string())
            })?;
        }
    }
    std::fs::create_dir_all(app_dir).map_err(|cause| {
        Error::refused("file.notWritten")
            .param("path", app_dir.display().to_string())
            .param("cause", cause.to_string())
    })?;
    std::fs::write(
        &marker,
        "Made by kilna --demo-all: wiped and made again by it.\n",
    )
    .map_err(|cause| {
        Error::refused("file.notWritten")
            .param("path", marker.display().to_string())
            .param("cause", cause.to_string())
    })?;

    let app = crate::db::open_app(&database)?;
    let mut made = Vec::new();
    for profile in crate::profile::builtin()? {
        if profile.key == crate::profile::BLANK_KIND {
            continue;
        }
        let space = crate::space::create(
            &app,
            app_dir,
            &crate::space::NewSpace {
                name: format!("Demo - {}", profile.name),
                kind: profile.key.clone(),
                colour: None,
                mark: None,
                type_id: None,
            },
            None,
        )?;
        let folder = crate::space::registry::folder(&app, app_dir, &space.id)?;
        let (conn, _) = crate::state::open_space(&folder, crate::state::Role::Companion)?;
        made.push((space.name.clone(), seed(&conn)?));
    }
    Ok(made)
}

/// Seed the demo into one space of an application folder: the one named, or
/// the one kilna last opened - when it holds no works yet.
pub fn seed_space(app_dir: &std::path::Path, name: Option<&str>) -> Result<(String, DemoReport)> {
    use crate::space::registry;
    let app = crate::db::open_app(&app_dir.join(crate::db::APP_DATABASE_FILE))?;
    let found = match name {
        Some(name) => registry::by_name(&app, app_dir, name)?,
        None => registry::newest(&app, app_dir)?.ok_or_else(|| Error::refused("space.none"))?,
    };
    let folder = registry::folder(&app, app_dir, &found.id)?;
    let (conn, held_by) = crate::state::open_space(&folder, crate::state::Role::Companion)?;
    if held_by.is_some() {
        return Err(Error::refused("space.unreachable").param("path", found.path.clone()));
    }
    Ok((found.name, seed(&conn)?))
}

/// The kind a craft writes in: the first one with a role to write into.
fn first_writing_kind(config: &ProfileConfig) -> Option<String> {
    config
        .work_kinds
        .iter()
        .find(|kind| !config.vocabulary(&kind.key).version_roles.is_empty() && kind.lab.is_none())
        .or_else(|| config.work_kinds.first())
        .map(|kind| kind.key.clone())
}

/// A judgement of the finished piece: every axis a little above the middle
/// of its scale, said to be the demo's.
fn score(conn: &Connection, config: &ProfileConfig, kind: &str, work_id: &str) -> Result<()> {
    let axes: Map<String, Value> = config
        .vocabulary(kind)
        .axes
        .iter()
        .map(|axis| {
            let value = (axis.scale * 0.7).round();
            (axis.key.clone(), json!(value))
        })
        .collect();
    if axes.is_empty() {
        return Ok(());
    }
    actions::score::create(
        conn,
        work_id,
        NewScore {
            axes,
            version_id: None,
            note: Some("A demo score: every axis a little above the middle.".into()),
            rater: None,
        },
    )?;
    Ok(())
}

/// A day `days` from today, as the calendar writes it.
fn days_from_today(days: i64) -> String {
    let day = time::OffsetDateTime::now_utc().date() + time::Duration::days(days);
    day.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, profile};

    fn space(kind: &str) -> Connection {
        let conn = db::open_in_memory().unwrap();
        profile::seed(&conn, kind).unwrap();
        conn
    }

    #[test]
    fn every_shipped_craft_gets_a_demo_with_something_on_each_screen() {
        for kind in ["music", "novel", "podcast", "blog", "poetry", "newsletter"] {
            let conn = space(kind);
            let report = seed(&conn).unwrap();
            assert!(report.works >= 3, "{kind}: {report:?}");
            assert!(report.notes >= 2, "{kind}: {report:?}");
            let scored: i64 = conn
                .query_row("SELECT count(*) FROM work_score", [], |row| row.get(0))
                .unwrap();
            assert_eq!(scored, 1, "{kind}: one finished piece is judged");
            let stages: Vec<i64> = conn
                .prepare("SELECT DISTINCT stage FROM work ORDER BY stage")
                .unwrap()
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap();
            assert!(
                stages.contains(&0) && stages.contains(&100),
                "{kind}: {stages:?}"
            );
        }
    }

    #[test]
    fn studio_s_demo_books_its_releases_in_the_coming_days() {
        let conn = space("music");
        let report = seed(&conn).unwrap();
        assert!(report.releases >= 1, "{report:?}");
        let today = time::OffsetDateTime::now_utc().date().to_string();
        let booked: Vec<String> = conn
            .prepare("SELECT scheduled_at FROM release WHERE scheduled_at IS NOT NULL")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(!booked.is_empty());
        assert!(
            booked.iter().all(|day| day.as_str() > today.as_str()),
            "{booked:?}"
        );
    }

    #[test]
    fn a_stand_holds_one_demo_space_of_every_craft_and_is_made_again_whole() {
        let dir = tempfile::tempdir().unwrap();
        let made = stand(dir.path()).unwrap();
        let shipped = profile::builtin().unwrap().len() - 1;
        assert_eq!(made.len(), shipped, "{made:?}");
        assert!(made.iter().all(|(_, report)| report.works >= 3));
        assert!(dir.path().join(STAND_MARKER).is_file());

        // Made again: wiped first, never doubled.
        let again = stand(dir.path()).unwrap();
        assert_eq!(again.len(), shipped);
        let app = crate::db::open_app(&dir.path().join(crate::db::APP_DATABASE_FILE)).unwrap();
        let listed = crate::space::registry::list(&app, dir.path(), None).unwrap();
        assert_eq!(listed.len(), shipped);
    }

    #[test]
    fn a_folder_of_someone_s_spaces_is_never_made_a_stand() {
        let dir = tempfile::tempdir().unwrap();
        let state = crate::state::AppState::start(dir.path(), None).unwrap();
        drop(state);
        assert_eq!(
            stand(dir.path()).unwrap_err().refusal().map(|r| r.code),
            Some("demo.notAStand")
        );
        assert!(
            dir.path().join(crate::space::SPACES_DIR).is_dir(),
            "nothing was wiped"
        );
    }

    #[test]
    fn a_demo_comes_in_russian_for_the_window_s_second_language() {
        for kind in ["music", "novel", "podcast", "blog", "poetry", "newsletter"] {
            let conn = space(kind);
            seed_in(&conn, "ru").unwrap();
            let title: String = conn
                .query_row(
                    "SELECT title FROM work ORDER BY created_at, rowid LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(
                title
                    .chars()
                    .any(|c| ('\u{0400}'..='\u{04FF}').contains(&c)),
                "{kind}: {title}"
            );
        }
    }

    #[test]
    fn a_demo_is_never_mixed_into_a_space_with_works() {
        let conn = space("blog");
        seed(&conn).unwrap();
        assert_eq!(
            seed(&conn).unwrap_err().refusal().map(|r| r.code),
            Some("demo.notEmpty")
        );
    }
}
