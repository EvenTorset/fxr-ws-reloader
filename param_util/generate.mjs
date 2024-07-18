import fs from 'node:fs'
import path from 'node:path'

console.log('Generating param_util.hpp...')

const paramDir = path.join(import.meta.dirname, '../libER/include/param')

const paramlist = Object.fromEntries(
  Array.from(
    (await fs.promises.readFile(path.join(paramDir, 'detail/paramlist.inl'), 'utf-8'))
      .matchAll(/LIBER_PARAM_ENTRY\((\w+), (\w+)\)/g)
  ).map(m => [m[2], m[1]])
)

let fieldMapTypes = ''
let fieldMaps = ''

const noCast = {
  int: true,
  float: true,
  bool: true,
}

const rowFields = {}

for (const fn of fs.readdirSync(path.join(paramDir, 'paramdef'))) {
  const structname = fn.slice(0, -4)
  if (!(structname in paramlist)) continue;
  const nssn = `from::paramdef::${structname}`
  const pd = await fs.promises.readFile(path.join(paramDir, 'paramdef', fn), 'utf-8')
  const matches = pd.matchAll(/^\s+((?:(?:un)?signed )?(?:int|float|char|short|bool)) (\w+)(?: : (\d+))?[^\[]*?;/gm)
  const fields = Array.from(matches).map(m => ({
    type: m[1],
    name: m[2]
  }))
  rowFields[paramlist[structname]] = fields
  fieldMapTypes += `

template <>
struct ParamFieldMap<${nssn}> {
  static const std::unordered_map<std::string, std::function<void(${nssn}&, const std::any&)>> setterMap;
};

  `.trim() + '\n'
  fieldMaps += `

const std::unordered_map<std::string, std::function<void(${nssn}&, const std::any&)>> ParamFieldMap<${nssn}>::setterMap = {
  ${fields.map(field =>
    `{"${field.name}", [](${nssn}& row, const std::any& value) { row.${field.name} = ${
      (field.type in noCast) ?
        `std::any_cast<${field.type}>(value)` :
        `static_cast<${field.type}>(std::any_cast<int>(value))`
    }; }}`
  ).join(',\n  ')}
};

  `.trim() + '\n'
}

function naturalSorter(as, bs) {
  let a, b, a1, b1, i = 0, n, L,
  rx = /(\.\d+)|(\d+(\.\d+)?)|([^\d.]+)|(\.\D+)|(\.$)/g
  if (as === bs) {
    return 0
  }
  if (typeof as !== 'string') {
    a = as.toString().toLowerCase().match(rx)
  } else {
    a = as.toLowerCase().match(rx)
  }
  if (typeof bs !== 'string') {
    b = bs.toString().toLowerCase().match(rx)
  } else {
    b = bs.toLowerCase().match(rx)
  }
  L = a.length
  while (i < L) {
    if (!b[i]) return 1
    a1 = a[i],
    b1 = b[i++]
    if (a1 !== b1) {
      n = a1 - b1
      if (!isNaN(n)) return n
      return a1 > b1 ? 1 : -1
    }
  }
  return b[i] ? -1 : 0
}

const out = `
/*
  Generated by param_util/generate.mjs
*/

#include <any>
#include <unordered_map>
#include <string>
#include <functional>
#include <variant>
#include <stdexcept>
#include <param/param.hpp>
#include <nlohmann/json.hpp>
using json = nlohmann::json;

template <typename T>
struct ParamFieldMap;
${fieldMapTypes}
${fieldMaps}
template <typename StructType>
void setParamFieldValue(StructType& obj, const std::string& memberName, const std::any& value) {
  const auto& setterMap = ParamFieldMap<StructType>::setterMap;
  auto it = setterMap.find(memberName);
  if (it != setterMap.end()) {
    it->second(obj, value);
  } else {
    throw std::invalid_argument("Field not found");
  }
}
std::any jsonToAny(const nlohmann::json& value) {
  if (value.is_number_integer()) {
    return value.get<int>();
  } else if (value.is_number_float()) {
    return static_cast<float>(value.get<double>());
  } else if (value.is_boolean()) {
    return value.get<bool>();
  } else {
    throw std::runtime_error("Unsupported JSON type");
  }
}
struct ParamRowActions {
  std::function<void(int, json)> modify;
  std::function<std::vector<int>()> listRows;
  std::function<json(int)> rowJSON;
};
const std::unordered_map<std::string, ParamRowActions> paramRowActionsMap = {
  ${Object.entries(paramlist).map(([type, name]) => `{"${name}", {
    [](int rowID, json fields) {
      auto [row, row_exists] = from::param::${name}[rowID];
      if (row_exists) for (auto& [field, value] : fields.items()) {
        setParamFieldValue(row, field, jsonToAny(value));
      }
    },
    []() {
      std::vector<int> ids;
      for (const auto& [id, row] : from::param::${name}) { ids.push_back(id); }
      return ids;
    },
    [](int rowID) {
      auto [row, row_exists] = from::param::${name}[rowID];
      if (row_exists) {
        return json{
          ${rowFields[name].map(field => `{"${field.name}", ${
            field.type in noCast ? `static_cast<${field.type === 'float' ? 'double' : field.type}>(row.${field.name})` : `static_cast<int>(row.${field.name})`
          }}`).join(',\n          ')}
        };
      } else {
        throw std::invalid_argument("Row not found");
      }
    }
  }}`).join(',\n  ')}
};
const json paramNameList = {
  ${Object.values(paramlist).sort(naturalSorter).map(e => `"${e}"`).join(',\n  ')}
};
`

await fs.promises.writeFile(path.join(import.meta.dirname, '../param_util.hpp'), out)