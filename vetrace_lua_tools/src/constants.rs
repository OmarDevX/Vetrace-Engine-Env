pub(crate) const KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto",
    "if", "in", "local", "nil", "not", "or", "repeat", "return", "then", "true", "until",
    "while",
];

pub(crate) const BUILTINS: &[&str] = &[
    "assert", "collectgarbage", "error", "getmetatable", "ipairs", "next", "pairs", "pcall",
    "print", "rawequal", "rawget", "rawlen", "rawset", "require", "select", "setmetatable",
    "tonumber", "tostring", "type", "warn", "xpcall", "math", "string", "table", "utf8",
    "coroutine", "Input", "Scene", "Entity", "Time", "Debug", "Physics", "Audio",
    "Assets", "Events", "Vec2", "Vec3", "Vec4", "Quat", "Color",
];
