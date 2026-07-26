export interface DatabaseTypeInfo {
  label: string;
  description: string;
}

const TYPE_INFO: Array<[RegExp, DatabaseTypeInfo]> = [
  [
    /\bobjectid\b/,
    {
      label: "文档唯一标识",
      description:
        "MongoDB 常用的 12 字节唯一标识，默认 _id 字段通常使用此类型。",
    },
  ],
  [
    /\b(document|object)\b/,
    {
      label: "嵌套文档",
      description:
        "一个字段中保存另一组键值数据，适合表达地址、配置等有层级的结构。",
    },
  ],
  [
    /\b(string)\b/,
    {
      label: "字符串",
      description:
        "MongoDB 的 UTF-8 文本类型，适合名称、标题、说明和普通文字内容。",
    },
  ],
  [
    /\b(long)\b/,
    {
      label: "64 位整数",
      description:
        "MongoDB 的 64 位有符号整数，适合较大的计数、编号或整数时间戳。",
    },
  ],
  [
    /\b(double)\b/,
    {
      label: "双精度浮点数",
      description:
        "MongoDB 的近似小数类型，适合测量值与统计值，不适合要求绝对精确的金额。",
    },
  ],
  [
    /\b(decimal)\b/,
    {
      label: "128 位精确小数",
      description:
        "MongoDB Decimal128 精确小数，适合金额、费率等需要避免浮点误差的数据。",
    },
  ],
  [
    /\b(binary)\b/,
    {
      label: "二进制数据",
      description:
        "MongoDB 的原始字节数据，可用于保存摘要、小文件或应用自定义的二进制内容。",
    },
  ],
  [
    /\b(regex)\b/,
    {
      label: "正则表达式",
      description:
        "MongoDB 可直接保存的正则表达式值，常用于模式匹配查询。",
    },
  ],
  [
    /\b(null)\b/,
    {
      label: "空值",
      description:
        "表示字段存在但没有具体值；它与字段完全不存在是两种不同状态。",
    },
  ],
  [
    /\btinyint\s*\(\s*1\s*\)/,
    {
      label: "布尔值/小整数",
      description:
        "MySQL 中常用 0 和 1 表示否与是，也可以存储很小范围的整数。",
    },
  ],
  [
    /\b(bigint|bigserial|int8)\b/,
    {
      label: "大整数",
      description:
        "用于存储范围很大的整数，常见于大型数据表主键、计数器和毫秒级时间戳。",
    },
  ],
  [
    /\b(smallint|smallserial|int2|tinyint)\b/,
    {
      label: "小整数",
      description:
        "用于存储范围较小的整数，适合状态码、等级、开关和小范围计数。",
    },
  ],
  [
    /\b(mediumint)\b/,
    {
      label: "中等整数",
      description:
        "MySQL 的三字节整数类型，取值范围介于 SMALLINT 和 INT 之间。",
    },
  ],
  [
    /\b(integer|int|serial|int4)\b/,
    {
      label: "整数",
      description:
        "用于存储没有小数部分的数值，常见于编号、数量、状态码和普通计数。",
    },
  ],
  [
    /\b(decimal|numeric|dec|fixed)\b/,
    {
      label: "精确小数",
      description:
        "按指定精度保存十进制数，不产生浮点舍入误差，适合金额、比例和财务数据。",
    },
  ],
  [
    /\b(double precision|double|float8)\b/,
    {
      label: "双精度浮点数",
      description:
        "用于科学计算或允许近似值的小数，范围大、精度高，但不适合精确金额计算。",
    },
  ],
  [
    /\b(real|float4|float)\b/,
    {
      label: "浮点数",
      description:
        "用于保存带小数的近似数值，适合测量值和统计值，不保证十进制绝对精确。",
    },
  ],
  [
    /\b(character varying|varchar|nvarchar)\b/,
    {
      label: "可变长度文本",
      description:
        "用于存储长度不固定的文本，只占用实际内容所需空间，适合名称、标题和普通字符串。",
    },
  ],
  [
    /\b(character|char|nchar|bpchar)\b/,
    {
      label: "固定长度文本",
      description:
        "文本长度固定，不足部分通常会补空格，适合长度始终一致的代码或标识。",
    },
  ],
  [
    /\b(longtext|mediumtext|tinytext|text|citext)\b/,
    {
      label: "长文本",
      description:
        "用于保存长度较大或不确定的文字内容，例如文章正文、备注、日志和描述信息。",
    },
  ],
  [
    /\b(enum)\b/,
    {
      label: "枚举",
      description:
        "值只能从预先定义的选项中选择，适合状态、类型等固定集合。",
    },
  ],
  [
    /\b(set)\b/,
    {
      label: "选项集合",
      description:
        "MySQL 中可同时选择多个预定义值的类型，适合保存少量固定标签组合。",
    },
  ],
  [
    /\b(boolean|bool)\b/,
    {
      label: "布尔值",
      description: "只表达真或假两种状态，常用于是否启用、是否删除等开关字段。",
    },
  ],
  [
    /\b(timestamp with time zone|timestamptz)\b/,
    {
      label: "带时区日期时间",
      description:
        "保存一个确定的时间点，并在读取时按会话时区显示，适合跨时区业务时间。",
    },
  ],
  [
    /\b(timestamp|datetime)\b/,
    {
      label: "日期时间",
      description:
        "同时保存日期和时间，常用于创建时间、更新时间、登录时间和事件发生时间。",
    },
  ],
  [
    /\b(date)\b/,
    {
      label: "日期",
      description: "只保存年、月、日，不包含具体时刻，适合生日、账期和自然日。",
    },
  ],
  [
    /\b(time)\b/,
    {
      label: "时间",
      description: "保存一天中的时、分、秒，不包含日期，适合营业时间和每日计划。",
    },
  ],
  [
    /\b(interval)\b/,
    {
      label: "时间间隔",
      description: "表示两个时间点之间的时长，例如几天、几小时或几个月。",
    },
  ],
  [
    /\b(year)\b/,
    {
      label: "年份",
      description: "MySQL 的年份类型，只保存年份，适合年度统计和生产年份。",
    },
  ],
  [
    /\b(jsonb)\b/,
    {
      label: "二进制 JSON",
      description:
        "PostgreSQL 的可索引 JSON 类型，适合结构灵活且需要按内部字段查询的数据。",
    },
  ],
  [
    /\b(json)\b/,
    {
      label: "JSON 文档",
      description:
        "用于保存对象或数组形式的结构化数据，适合字段结构不固定的扩展信息。",
    },
  ],
  [
    /\b(uuid)\b/,
    {
      label: "全局唯一标识",
      description:
        "保存标准 UUID，适合分布式场景下生成不易重复的主键或业务标识。",
    },
  ],
  [
    /\b(bytea|longblob|mediumblob|tinyblob|blob|varbinary|binary)\b/,
    {
      label: "二进制数据",
      description:
        "用于保存原始字节，例如图片、文件、摘要或加密内容；界面通常不会直接显示其文本含义。",
    },
  ],
  [
    /\b(bit varying|varbit|bit)\b/,
    {
      label: "位串",
      description: "按二进制位保存数据，适合权限位、掩码和紧凑的开关集合。",
    },
  ],
  [
    /\b(inet|cidr)\b/,
    {
      label: "网络地址",
      description:
        "PostgreSQL 的 IP 地址或网络段类型，可以校验地址格式并支持网络范围运算。",
    },
  ],
  [
    /\b(macaddr8|macaddr)\b/,
    {
      label: "MAC 地址",
      description: "PostgreSQL 的硬件地址类型，用于保存并校验网卡 MAC 地址。",
    },
  ],
  [
    /\b(xml)\b/,
    {
      label: "XML 文档",
      description: "用于保存 XML 格式的结构化文本，并保证内容符合基本 XML 语法。",
    },
  ],
  [
    /\b(geometry|geography|point|line|polygon)\b/,
    {
      label: "空间数据",
      description: "用于保存坐标、点、线或区域等地理空间信息。",
    },
  ],
  [
    /\b(oid|regclass|regtype|xid|xid8|cid|tid|pg_lsn|name)\b/,
    {
      label: "PostgreSQL 系统标识",
      description:
        "PostgreSQL 内部使用的对象、事务或日志位置标识。系统目录中很常见，不建议在不了解依赖关系时直接修改。",
    },
  ],
  [
    /\b(array)\b|\[\]$/,
    {
      label: "数组",
      description:
        "一个字段中保存多个同类型值，适合数量有限且通常一起读写的数据集合。",
    },
  ],
];

export function databaseTypeInfo(rawType: string): DatabaseTypeInfo {
  const normalized = rawType.trim().toLowerCase();
  const matched = TYPE_INFO.find(([pattern]) => pattern.test(normalized));
  if (matched) return matched[1];

  return {
    label: "数据库原生类型",
    description: `这是数据库提供的 ${rawType} 类型。智屿暂时没有更具体的中文说明，修改系统表字段前请先确认数据库文档。`,
  };
}
