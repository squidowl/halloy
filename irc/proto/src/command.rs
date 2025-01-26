#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /* Connection Messages */
    /// [*] <subcommand> [*] [<param>]
    CAP(Option<String>, String, Option<String>, Option<String>),
    /// <data>
    AUTHENTICATE(String),
    /// <password>
    PASS(String),
    /// <nickname>
    NICK(String),
    /// <username> <realname>
    USER(String, String),
    /// <token>
    PING(String),
    /// [<server>] <token>
    PONG(String, Option<String>),
    /// <name> <password>
    OPER(String, String),
    /// [<reason>]
    QUIT(Option<String>),
    /// <reason>
    ERROR(String),

    /* Channel Operations */
    /// <channel>{,<channel>} [<key>{,<key>}] (send)
    /// <channel>{,<channel>} [<accountname>] (receive [extended-join])
    JOIN(String, Option<String>),
    /// <channel>{,<channel>} [<reason>]
    PART(String, Option<String>),
    /// <channel> [<topic>]
    TOPIC(String, Option<String>),
    /// <channel>{,<channel>}
    NAMES(String),
    /// [<channel>{,<channel>}] [<elistcond>{,<elistcond>}]
    LIST(Option<String>, Option<String>),
    /// <nickname> <channel>
    INVITE(String, String),
    /// <channel> [<user>{,<user>}] [<comment>]
    KICK(String, String, Option<String>),

    /* Server Queries and Commands */
    /// [<target>]
    MOTD(Option<String>),
    /// [<target>]
    VERSION(Option<String>),

    /// [<target>]
    ADMIN(Option<String>),
    /// <target server> [<port> [<remote server>]]
    CONNECT(String, Option<String>, Option<String>),
    /// None
    LUSERS,
    /// [<server>]
    TIME(Option<String>),
    /// <query> [<server>]
    STATS(String, Option<String>),
    /// [<subject>]
    HELP(Option<String>),
    /// None
    INFO,
    /// <target> [<modestring> [<mode arguments>...]]
    MODE(String, Option<String>, Option<Vec<String>>),

    /* Sending Messages */
    /// <target>{,<target>} <text to be sent>
    PRIVMSG(String, String),
    /// <target>{,<target>} <text to be sent>
    NOTICE(String, String),

    /* User-Based Queries */
    /// <mask> [%<fields>[,<token>]]
    WHO(String, Option<String>, Option<String>),
    /// [<target>] <nick>
    WHOIS(Option<String>, String),
    /// <nick> [<count>]
    WHOWAS(String, Option<String>),
    /// <nickname> <comment>
    KILL(String, String),
    /// None
    REHASH,
    /// None
    RESTART,
    /// <server> <comment>
    SQUIT(String, String),

    /* Optional Messages */
    /// [<text>]
    AWAY(Option<String>),
    /// None
    LINKS,
    /// <nickname>{ <nickname>}
    USERHOST(Vec<String>),
    /// <text>
    WALLOPS(String),

    /// <accountname>
    ACCOUNT(String),
    /* IRC extensions */
    /// <type> [<params>]
    BATCH(String, Vec<String>),
    /// <subcommand> [<params>]
    CHATHISTORY(String, Vec<String>),
    /// <new_username> <new_hostname>
    CHGHOST(String, String),
    /// <nickname> <channel> :<message>
    CNOTICE(String, String, String),
    /// <nickname> <channel> :<message>
    CPRIVMSG(String, String, String),
    /// <channel> [<message>]
    KNOCK(String, Option<String>),
    /// <target> [<timestamp>]
    MARKREAD(String, Option<String>),
    /// <subcommand> [<targets>]
    MONITOR(String, Option<String>),
    /// <realname>
    SETNAME(String),
    /// <msgtarget>
    TAGMSG(String),
    /// <nickname>
    USERIP(String),

    /* Standard Replies */
    /// <command> <code> [<context>] <description>
    FAIL(String, String, Option<Vec<String>>, String),
    /// <command> <code> [<context>] <description>
    WARN(String, String, Option<Vec<String>>, String),
    /// <command> <code> [<context>] <description>
    NOTE(String, String, Option<Vec<String>>, String),

    Numeric(Numeric, Vec<String>),
    Unknown(String, Vec<String>),
    Raw(String),
}

impl Command {
    pub fn new(tag: &str, parameters: Vec<String>) -> Self {
        use Command::*;

        if let Ok(num) = tag.parse::<u16>() {
            return match self::Numeric::try_from(num) {
                Ok(numeric) => Numeric(numeric, parameters),
                Err(_) => Unknown(num.to_string(), parameters),
            };
        }

        let tag = tag.to_uppercase();
        let len = parameters.len();

        let mut params = parameters.into_iter();

        macro_rules! req {
            () => {
                params.next().unwrap()
            };
        }
        macro_rules! opt {
            () => {
                params.next()
            };
        }

        match tag.as_str() {
            "CAP" if len > 0 => {
                let a = req!();
                match opt!() {
                    Some(b) => CAP(Some(a), b, opt!(), opt!()),
                    None => CAP(None, a, None, None),
                }
            }
            "AUTHENTICATE" if len > 0 => AUTHENTICATE(req!()),
            "PASS" if len > 0 => PASS(req!()),
            "NICK" if len > 0 => NICK(req!()),
            "USER" if len > 1 => USER(req!(), req!()),
            "PING" if len > 0 => PING(req!()),
            "PONG" if len > 0 => PONG(req!(), opt!()),
            "OPER" if len > 1 => OPER(req!(), req!()),
            "QUIT" => QUIT(opt!()),
            "ERROR" if len > 0 => ERROR(req!()),
            "JOIN" if len > 0 => JOIN(req!(), opt!()),
            "PART" if len > 0 => PART(req!(), opt!()),
            "TOPIC" if len > 0 => TOPIC(req!(), opt!()),
            "NAMES" if len > 0 => NAMES(req!()),
            "LIST" => LIST(opt!(), opt!()),
            "INVITE" if len > 1 => INVITE(req!(), req!()),
            "KICK" if len > 1 => KICK(req!(), req!(), opt!()),
            "MOTD" => MOTD(opt!()),
            "VERSION" => VERSION(opt!()),
            "ADMIN" => ADMIN(opt!()),
            "CONNECT" if len > 0 => CONNECT(req!(), opt!(), opt!()),
            "LUSERS" => LUSERS,
            "TIME" => TIME(opt!()),
            "STATS" if len > 0 => STATS(req!(), opt!()),
            "HELP" => HELP(opt!()),
            "INFO" => INFO,
            "MODE" if len > 0 => MODE(req!(), opt!(), Some(params.collect())),
            "PRIVMSG" if len > 1 => PRIVMSG(req!(), req!()),
            "NOTICE" if len > 1 => NOTICE(req!(), req!()),
            "WHO" if len > 0 => WHO(req!(), opt!(), opt!()),
            "WHOIS" => {
                let a = req!();
                match opt!() {
                    Some(b) => WHOIS(Some(a), b),
                    None => WHOIS(None, a),
                }
            }
            "WHOWAS" if len > 0 => WHOWAS(req!(), opt!()),
            "KILL" if len > 1 => KILL(req!(), req!()),
            "REHASH" => REHASH,
            "RESTART" => RESTART,
            "SQUIT" if len > 1 => SQUIT(req!(), req!()),
            "AWAY" => AWAY(opt!()),
            "LINKS" => LINKS,
            "USERHOST" => USERHOST(params.collect()),
            "WALLOPS" if len > 0 => WALLOPS(req!()),
            "ACCOUNT" if len > 0 => ACCOUNT(req!()),
            "BATCH" if len > 0 => BATCH(req!(), params.collect()),
            "CHATHISTORY" if len > 0 => CHATHISTORY(req!(), params.collect()),
            "CHGHOST" if len > 1 => CHGHOST(req!(), req!()),
            "CNOTICE" if len > 2 => CNOTICE(req!(), req!(), req!()),
            "CPRIVMSG" if len > 2 => CPRIVMSG(req!(), req!(), req!()),
            "KNOCK" if len > 0 => KNOCK(req!(), opt!()),
            "MARKREAD" if len > 0 => MARKREAD(req!(), opt!()),
            "MONITOR" if len > 0 => MONITOR(req!(), opt!()),
            "SETNAME" if len > 0 => SETNAME(req!()),
            "TAGMSG" if len > 0 => TAGMSG(req!()),
            "USERIP" if len > 0 => USERIP(req!()),
            "FAIL" if len > 2 => {
                let a = req!();
                let b = req!();
                let mut c: Vec<String> = params.collect();
                let d = c.pop().unwrap();
                if !c.is_empty() {
                    FAIL(a, b, Some(c), d)
                } else {
                    FAIL(a, b, None, d)
                }
            }
            "WARN" if len > 2 => {
                let a = req!();
                let b = req!();
                let mut c: Vec<String> = params.collect();
                let d = c.pop().unwrap();
                if !c.is_empty() {
                    WARN(a, b, Some(c), d)
                } else {
                    WARN(a, b, None, d)
                }
            }
            "NOTE" if len > 2 => {
                let a = req!();
                let b = req!();
                let mut c: Vec<String> = params.collect();
                let d = c.pop().unwrap();
                if !c.is_empty() {
                    NOTE(a, b, Some(c), d)
                } else {
                    NOTE(a, b, None, d)
                }
            }
            _ => Self::Unknown(tag, params.collect()),
        }
    }

    pub fn parameters(self) -> Vec<String> {
        match self {
            Command::CAP(a, b, c, d) => a.into_iter().chain(Some(b)).chain(c).chain(d).collect(),
            Command::AUTHENTICATE(a) => vec![a],
            Command::PASS(a) => vec![a],
            Command::NICK(a) => vec![a],
            Command::USER(a, b) => vec![a, "0".into(), "*".into(), b],
            Command::PING(a) => vec![a],
            Command::PONG(a, b) => std::iter::once(a).chain(b).collect(),
            Command::OPER(a, b) => vec![a, b],
            Command::QUIT(a) => a.into_iter().collect(),
            Command::ERROR(a) => vec![a],
            Command::JOIN(a, b) => std::iter::once(a).chain(b).collect(),
            Command::PART(a, b) => std::iter::once(a).chain(b).collect(),
            Command::TOPIC(a, b) => std::iter::once(a).chain(b).collect(),
            Command::NAMES(a) => vec![a],
            Command::LIST(a, b) => a.into_iter().chain(b).collect(),
            Command::INVITE(a, b) => vec![a, b],
            Command::KICK(a, b, c) => std::iter::once(a).chain(Some(b)).chain(c).collect(),
            Command::MOTD(a) => a.into_iter().collect(),
            Command::VERSION(a) => a.into_iter().collect(),
            Command::ADMIN(a) => a.into_iter().collect(),
            Command::CONNECT(a, b, c) => std::iter::once(a).chain(b).chain(c).collect(),
            Command::LUSERS => vec![],
            Command::TIME(a) => a.into_iter().collect(),
            Command::STATS(a, b) => std::iter::once(a).chain(b).collect(),
            Command::HELP(a) => a.into_iter().collect(),
            Command::INFO => vec![],
            Command::MODE(a, b, c) => std::iter::once(a)
                .chain(b)
                .chain(c.into_iter().flatten())
                .collect(),
            Command::PRIVMSG(a, b) => vec![a, b],
            Command::NOTICE(a, b) => vec![a, b],
            Command::WHO(a, b, c) => std::iter::once(a)
                .chain(b.map(|b| c.map_or_else(|| format!("%{}", b), |c| format!("%{},{}", b, c))))
                .collect(),
            Command::WHOIS(a, b) => a.into_iter().chain(Some(b)).collect(),
            Command::WHOWAS(a, b) => std::iter::once(a).chain(b).collect(),
            Command::KILL(a, b) => vec![a, b],
            Command::REHASH => vec![],
            Command::RESTART => vec![],
            Command::SQUIT(a, b) => vec![a, b],
            Command::AWAY(a) => a.into_iter().collect(),
            Command::LINKS => vec![],
            Command::USERHOST(params) => params,
            Command::WALLOPS(a) => vec![a],
            Command::ACCOUNT(a) => vec![a],
            Command::BATCH(a, rest) => std::iter::once(a).chain(rest).collect(),
            Command::CHATHISTORY(a, b) => std::iter::once(a).chain(b).collect(),
            Command::CHGHOST(a, b) => vec![a, b],
            Command::CNOTICE(a, b, c) => vec![a, b, c],
            Command::CPRIVMSG(a, b, c) => vec![a, b, c],
            Command::KNOCK(a, b) => std::iter::once(a).chain(b).collect(),
            Command::MARKREAD(a, b) => std::iter::once(a).chain(b).collect(),
            Command::MONITOR(a, b) => std::iter::once(a).chain(b).collect(),
            Command::SETNAME(a) => vec![a],
            Command::TAGMSG(a) => vec![a],
            Command::USERIP(a) => vec![a],
            Command::FAIL(a, b, c, d) => std::iter::once(a)
                .chain(Some(b))
                .chain(c.into_iter().flatten())
                .chain(Some(d))
                .collect(),
            Command::WARN(a, b, c, d) => std::iter::once(a)
                .chain(Some(b))
                .chain(c.into_iter().flatten())
                .chain(Some(d))
                .collect(),
            Command::NOTE(a, b, c, d) => std::iter::once(a)
                .chain(Some(b))
                .chain(c.into_iter().flatten())
                .chain(Some(d))
                .collect(),
            Command::Numeric(_, params) => params,
            Command::Unknown(_, params) => params,
            Command::Raw(_) => vec![],
        }
    }

    pub fn command(&self) -> String {
        use Command::*;

        match self {
            CAP(_, _, _, _) => "CAP".to_string(),
            AUTHENTICATE(_) => "AUTHENTICATE".to_string(),
            PASS(_) => "PASS".to_string(),
            NICK(_) => "NICK".to_string(),
            USER(_, _) => "USER".to_string(),
            PING(_) => "PING".to_string(),
            PONG(_, _) => "PONG".to_string(),
            OPER(_, _) => "OPER".to_string(),
            QUIT(_) => "QUIT".to_string(),
            ERROR(_) => "ERROR".to_string(),
            JOIN(_, _) => "JOIN".to_string(),
            PART(_, _) => "PART".to_string(),
            TOPIC(_, _) => "TOPIC".to_string(),
            NAMES(_) => "NAMES".to_string(),
            LIST(_, _) => "LIST".to_string(),
            INVITE(_, _) => "INVITE".to_string(),
            KICK(_, _, _) => "KICK".to_string(),
            MOTD(_) => "MOTD".to_string(),
            VERSION(_) => "VERSION".to_string(),
            ADMIN(_) => "ADMIN".to_string(),
            CONNECT(_, _, _) => "CONNECT".to_string(),
            LUSERS => "LUSERS".to_string(),
            TIME(_) => "TIME".to_string(),
            STATS(_, _) => "STATS".to_string(),
            HELP(_) => "HELP".to_string(),
            INFO => "INFO".to_string(),
            MODE(_, _, _) => "MODE".to_string(),
            PRIVMSG(_, _) => "PRIVMSG".to_string(),
            NOTICE(_, _) => "NOTICE".to_string(),
            WHO(_, _, _) => "WHO".to_string(),
            WHOIS(_, _) => "WHOIS".to_string(),
            WHOWAS(_, _) => "WHOWAS".to_string(),
            KILL(_, _) => "KILL".to_string(),
            REHASH => "REHASH".to_string(),
            RESTART => "RESTART".to_string(),
            SQUIT(_, _) => "SQUIT".to_string(),
            AWAY(_) => "AWAY".to_string(),
            LINKS => "LINKS".to_string(),
            USERHOST(_) => "USERHOST".to_string(),
            WALLOPS(_) => "WALLOPS".to_string(),
            ACCOUNT(_) => "ACCOUNT".to_string(),
            BATCH(_, _) => "BATCH".to_string(),
            CHATHISTORY(_, _) => "CHATHISTORY".to_string(),
            CHGHOST(_, _) => "CHGHOST".to_string(),
            CNOTICE(_, _, _) => "CNOTICE".to_string(),
            CPRIVMSG(_, _, _) => "CPRIVMSG".to_string(),
            KNOCK(_, _) => "KNOCK".to_string(),
            MARKREAD(_, _) => "MARKREAD".to_string(),
            MONITOR(_, _) => "MONITOR".to_string(),
            SETNAME(_) => "SETNAME".to_string(),
            TAGMSG(_) => "TAGMSG".to_string(),
            USERIP(_) => "USERIP".to_string(),
            FAIL(_, _, _, _) => "FAIL".to_string(),
            WARN(_, _, _, _) => "WARN".to_string(),
            NOTE(_, _, _, _) => "NOTE".to_string(),
            Numeric(numeric, _) => format!("{:03}", *numeric as u16),
            Unknown(tag, _) => tag.clone(),
            Raw(_) => "".to_string(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Numeric {
    RPL_WELCOME = 1,
    RPL_YOURHOST = 2,
    RPL_CREATED = 3,
    RPL_MYINFO = 4,
    RPL_ISUPPORT = 5,
    RPL_BOUNCE = 10,
    RPL_STATSCOMMANDS = 212,
    RPL_ENDOFSTATS = 219,
    RPL_STATSUPTIME = 242,
    RPL_UMODEIS = 221,
    RPL_LUSERCLIENT = 251,
    RPL_LUSEROP = 252,
    RPL_LUSERUNKNOWN = 253,
    RPL_LUSERCHANNELS = 254,
    RPL_LUSERME = 255,
    RPL_ADMINME = 256,
    RPL_ADMINLOC1 = 257,
    RPL_ADMINLOC2 = 258,
    RPL_ADMINEMAIL = 259,
    RPL_TRYAGAIN = 263,
    RPL_LOCALUSERS = 265,
    RPL_GLOBALUSERS = 266,
    RPL_WHOISCERTFP = 276,
    RPL_NONE = 300,
    RPL_AWAY = 301,
    RPL_USERHOST = 302,
    RPL_UNAWAY = 305,
    RPL_NOWAWAY = 306,
    RPL_WHOREPLY = 352,
    RPL_ENDOFWHO = 315,
    RPL_WHOISREGNICK = 307,
    RPL_WHOISUSER = 311,
    RPL_WHOISSERVER = 312,
    RPL_WHOISOPERATOR = 313,
    RPL_WHOWASUSER = 314,
    RPL_WHOISIDLE = 317,
    RPL_ENDOFWHOIS = 318,
    RPL_WHOISCHANNELS = 319,
    RPL_WHOISSPECIAL = 320,
    RPL_LISTSTART = 321,
    RPL_LIST = 322,
    RPL_LISTEND = 323,
    RPL_CHANNELMODEIS = 324,
    RPL_CREATIONTIME = 329,
    RPL_WHOISACCOUNT = 330,
    RPL_NOTOPIC = 331,
    RPL_TOPIC = 332,
    RPL_TOPICWHOTIME = 333,
    RPL_INVITELIST = 336,
    RPL_ENDOFINVITELIST = 337,
    RPL_WHOISACTUALLY = 338,
    RPL_INVITING = 341,
    RPL_INVEXLIST = 346,
    RPL_ENDOFINVEXLIST = 347,
    RPL_EXCEPTLIST = 348,
    RPL_ENDOFEXCEPTLIST = 349,
    RPL_VERSION = 351,
    RPL_NAMREPLY = 353,
    RPL_WHOSPCRPL = 354,
    RPL_ENDOFNAMES = 366,
    RPL_LINKS = 364,
    RPL_ENDOFLINKS = 365,
    RPL_BANLIST = 367,
    RPL_ENDOFBANLIST = 368,
    RPL_ENDOFWHOWAS = 369,
    RPL_INFO = 371,
    RPL_ENDOFINFO = 374,
    RPL_MOTDSTART = 375,
    RPL_MOTD = 372,
    RPL_ENDOFMOTD = 376,
    RPL_WHOISHOST = 378,
    RPL_WHOISMODES = 379,
    RPL_YOUREOPER = 381,
    RPL_REHASHING = 382,
    RPL_TIME = 391,
    ERR_UNKNOWNERROR = 400,
    ERR_NOSUCHNICK = 401,
    ERR_NOSUCHSERVER = 402,
    ERR_NOSUCHCHANNEL = 403,
    ERR_CANNOTSENDTOCHAN = 404,
    ERR_TOOMANYCHANNELS = 405,
    ERR_WASNOSUCHNICK = 406,
    ERR_NOORIGIN = 409,
    ERR_NORECIPIENT = 411,
    ERR_NOTEXTTOSEND = 412,
    ERR_INPUTTOOLONG = 417,
    ERR_UNKNOWNCOMMAND = 421,
    ERR_NOMOTD = 422,
    ERR_NONICKNAMEGIVEN = 431,
    ERR_ERRONEUSNICKNAME = 432,
    ERR_NICKNAMEINUSE = 433,
    ERR_NICKCOLLISION = 436,
    ERR_USERNOTINCHANNEL = 441,
    ERR_NOTONCHANNEL = 442,
    ERR_USERONCHANNEL = 443,
    ERR_NOTREGISTERED = 451,
    ERR_NEEDMOREPARAMS = 461,
    ERR_ALREADYREGISTERED = 462,
    ERR_PASSWDMISMATCH = 464,
    ERR_YOUREBANNEDCREEP = 465,
    ERR_CHANNELISFULL = 471,
    ERR_UNKNOWNMODE = 472,
    ERR_INVITEONLYCHAN = 473,
    ERR_BANNEDFROMCHAN = 474,
    ERR_BADCHANNELKEY = 475,
    ERR_BADCHANMASK = 476,
    ERR_NOCHANMODES = 477,
    ERR_NOPRIVILEGES = 481,
    ERR_CHANOPRIVSNEEDED = 482,
    ERR_CANTKILLSERVER = 483,
    ERR_NOOPERHOST = 491,
    ERR_UMODEUNKNOWNFLAG = 501,
    ERR_USERSDONTMATCH = 502,
    ERR_HELPNOTFOUND = 524,
    ERR_INVALIDKEY = 525,
    RPL_STARTTLS = 670,
    RPL_WHOISSECURE = 671,
    ERR_STARTTLS = 691,
    ERR_INVALIDMODEPARAM = 696,
    RPL_HELPSTART = 704,
    RPL_HELPTXT = 705,
    RPL_ENDOFHELP = 706,
    ERR_NOPRIVS = 723,
    RPL_MONONLINE = 730,
    RPL_MONOFFLINE = 731,
    RPL_MONLIST = 732,
    RPL_ENDOFMONLIST = 733,
    ERR_MONLISTFULL = 734,
    RPL_LOGGEDIN = 900,
    RPL_LOGGEDOUT = 901,
    ERR_NICKLOCKED = 902,
    RPL_SASLSUCCESS = 903,
    ERR_SASLFAIL = 904,
    ERR_SASLTOOLONG = 905,
    ERR_SASLABORTED = 906,
    ERR_SASLALREADY = 907,
    RPL_SASLMECHS = 908,
}

impl TryFrom<u16> for Numeric {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        use Numeric::*;

        Ok(match value {
            1 => RPL_WELCOME,
            2 => RPL_YOURHOST,
            3 => RPL_CREATED,
            4 => RPL_MYINFO,
            5 => RPL_ISUPPORT,
            10 => RPL_BOUNCE,
            212 => RPL_STATSCOMMANDS,
            219 => RPL_ENDOFSTATS,
            242 => RPL_STATSUPTIME,
            221 => RPL_UMODEIS,
            251 => RPL_LUSERCLIENT,
            252 => RPL_LUSEROP,
            253 => RPL_LUSERUNKNOWN,
            254 => RPL_LUSERCHANNELS,
            255 => RPL_LUSERME,
            256 => RPL_ADMINME,
            257 => RPL_ADMINLOC1,
            258 => RPL_ADMINLOC2,
            259 => RPL_ADMINEMAIL,
            263 => RPL_TRYAGAIN,
            265 => RPL_LOCALUSERS,
            266 => RPL_GLOBALUSERS,
            276 => RPL_WHOISCERTFP,
            300 => RPL_NONE,
            301 => RPL_AWAY,
            302 => RPL_USERHOST,
            305 => RPL_UNAWAY,
            306 => RPL_NOWAWAY,
            352 => RPL_WHOREPLY,
            315 => RPL_ENDOFWHO,
            307 => RPL_WHOISREGNICK,
            311 => RPL_WHOISUSER,
            312 => RPL_WHOISSERVER,
            313 => RPL_WHOISOPERATOR,
            314 => RPL_WHOWASUSER,
            317 => RPL_WHOISIDLE,
            318 => RPL_ENDOFWHOIS,
            319 => RPL_WHOISCHANNELS,
            320 => RPL_WHOISSPECIAL,
            321 => RPL_LISTSTART,
            322 => RPL_LIST,
            323 => RPL_LISTEND,
            324 => RPL_CHANNELMODEIS,
            329 => RPL_CREATIONTIME,
            330 => RPL_WHOISACCOUNT,
            331 => RPL_NOTOPIC,
            332 => RPL_TOPIC,
            333 => RPL_TOPICWHOTIME,
            336 => RPL_INVITELIST,
            337 => RPL_ENDOFINVITELIST,
            338 => RPL_WHOISACTUALLY,
            341 => RPL_INVITING,
            346 => RPL_INVEXLIST,
            347 => RPL_ENDOFINVEXLIST,
            348 => RPL_EXCEPTLIST,
            349 => RPL_ENDOFEXCEPTLIST,
            351 => RPL_VERSION,
            353 => RPL_NAMREPLY,
            354 => RPL_WHOSPCRPL,
            366 => RPL_ENDOFNAMES,
            364 => RPL_LINKS,
            365 => RPL_ENDOFLINKS,
            367 => RPL_BANLIST,
            368 => RPL_ENDOFBANLIST,
            369 => RPL_ENDOFWHOWAS,
            371 => RPL_INFO,
            374 => RPL_ENDOFINFO,
            375 => RPL_MOTDSTART,
            372 => RPL_MOTD,
            376 => RPL_ENDOFMOTD,
            378 => RPL_WHOISHOST,
            379 => RPL_WHOISMODES,
            381 => RPL_YOUREOPER,
            382 => RPL_REHASHING,
            391 => RPL_TIME,
            400 => ERR_UNKNOWNERROR,
            401 => ERR_NOSUCHNICK,
            402 => ERR_NOSUCHSERVER,
            403 => ERR_NOSUCHCHANNEL,
            404 => ERR_CANNOTSENDTOCHAN,
            405 => ERR_TOOMANYCHANNELS,
            406 => ERR_WASNOSUCHNICK,
            409 => ERR_NOORIGIN,
            411 => ERR_NORECIPIENT,
            412 => ERR_NOTEXTTOSEND,
            417 => ERR_INPUTTOOLONG,
            421 => ERR_UNKNOWNCOMMAND,
            422 => ERR_NOMOTD,
            431 => ERR_NONICKNAMEGIVEN,
            432 => ERR_ERRONEUSNICKNAME,
            433 => ERR_NICKNAMEINUSE,
            436 => ERR_NICKCOLLISION,
            441 => ERR_USERNOTINCHANNEL,
            442 => ERR_NOTONCHANNEL,
            443 => ERR_USERONCHANNEL,
            451 => ERR_NOTREGISTERED,
            461 => ERR_NEEDMOREPARAMS,
            462 => ERR_ALREADYREGISTERED,
            464 => ERR_PASSWDMISMATCH,
            465 => ERR_YOUREBANNEDCREEP,
            471 => ERR_CHANNELISFULL,
            472 => ERR_UNKNOWNMODE,
            473 => ERR_INVITEONLYCHAN,
            474 => ERR_BANNEDFROMCHAN,
            475 => ERR_BADCHANNELKEY,
            476 => ERR_BADCHANMASK,
            477 => ERR_NOCHANMODES,
            481 => ERR_NOPRIVILEGES,
            482 => ERR_CHANOPRIVSNEEDED,
            483 => ERR_CANTKILLSERVER,
            491 => ERR_NOOPERHOST,
            501 => ERR_UMODEUNKNOWNFLAG,
            502 => ERR_USERSDONTMATCH,
            524 => ERR_HELPNOTFOUND,
            525 => ERR_INVALIDKEY,
            670 => RPL_STARTTLS,
            671 => RPL_WHOISSECURE,
            691 => ERR_STARTTLS,
            696 => ERR_INVALIDMODEPARAM,
            704 => RPL_HELPSTART,
            705 => RPL_HELPTXT,
            706 => RPL_ENDOFHELP,
            723 => ERR_NOPRIVS,
            730 => RPL_MONONLINE,
            731 => RPL_MONOFFLINE,
            732 => RPL_MONLIST,
            733 => RPL_ENDOFMONLIST,
            734 => ERR_MONLISTFULL,
            900 => RPL_LOGGEDIN,
            901 => RPL_LOGGEDOUT,
            902 => ERR_NICKLOCKED,
            903 => RPL_SASLSUCCESS,
            904 => ERR_SASLFAIL,
            905 => ERR_SASLTOOLONG,
            906 => ERR_SASLABORTED,
            907 => ERR_SASLALREADY,
            908 => RPL_SASLMECHS,
            _ => return Err(()),
        })
    }
}
