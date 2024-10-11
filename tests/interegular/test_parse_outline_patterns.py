import pytest
from outlines_core.fsm.json_schema import (
    INTEGER,
    NULL,
    WHITESPACE,
    STRING,  #
    STRING_INNER,  #
    BOOLEAN,
    NUMBER,
)
from interegular.patterns import Pattern as InteregularPattern
from interegular.patterns import _CharGroup, _Concatenation, _Repeated
from outlines_core.fsm.regex import parse_pattern
import interegular


def convert_to_interegular(element):
    if isinstance(element, InteregularPattern):
        return element

    element_type = type(element).__name__

    if element_type == "PyLiteral":
        # TODO: handle the negated case if needed
        return _CharGroup(frozenset(element.value), negated=False)

    elif element_type == "PyCharGroup":
        return _CharGroup(frozenset(element.chars), negated=element.inverted)

    elif element_type == "PyRepeated":
        base = convert_to_interegular(element.element)
        return _Repeated(base, element.min, element.max)

    elif element_type == "PyConcatenation":
        parts = [convert_to_interegular(e) for e in element.elements]
        return _Concatenation(parts)

    elif element_type == "PyAlternation":
        options = [convert_to_interegular(e) for e in element.elements]
        return InteregularPattern(options)

    elif element_type == "PyCapture":
        # interegular doesn't have a direct equivalent for Capture
        # we'll just convert the inner element
        return convert_to_interegular(element.element)

    elif element_type == "PyGroup":
        # similar to Capture, we'll just convert the inner element
        return convert_to_interegular(element.element)

    elif element_type == "PyAnchor":
        # TODO: handle the different types of anchors if needed
        # interegular doesn't have a direct equivalent for Anchor either
        # in this case, we'll just raise an error
        raise NotImplementedError("Anchors are not supported in interegular")

    elif element_type == "PyFlag":
        return convert_to_interegular(element.element)

    else:
        raise ValueError(f"Unhandled element type: {element_type}")


def deep_compare(pattern1, pattern2):
    if isinstance(pattern1, InteregularPattern) != isinstance(
        pattern2, InteregularPattern
    ):
        return False

    if isinstance(pattern1, InteregularPattern):
        if len(pattern1.options) != len(pattern2.options):
            return False
        return all(
            deep_compare(opt1, opt2)
            for opt1, opt2 in zip(pattern1.options, pattern2.options)
        )

    elif isinstance(pattern1, _Concatenation):
        if len(pattern1.parts) != len(pattern2.parts):
            return False
        return all(
            deep_compare(elem1, elem2)
            for elem1, elem2 in zip(pattern1.parts, pattern2.parts)
        )

    elif isinstance(pattern1, _CharGroup):
        return pattern1.chars == pattern2.chars and pattern1.negated == pattern2.negated

    elif isinstance(pattern1, _Repeated):
        return (
            deep_compare(pattern1.base, pattern2.base)
            and pattern1.min == pattern2.min
            and pattern1.max == pattern2.max
        )

    else:
        raise ValueError(f"Unhandled pattern type: {type(pattern1)}")


# test parameters copied from tests/fsm/test_json_schema.py to align with the test
@pytest.mark.parametrize(
    "schema,regex,examples",
    [
        # # String
        # (
        #     {"title": "Foo", "type": "string"},
        #     STRING,
        #     [
        #         ("unquotedstring", False),
        #         ('"(parenthesized_string)"', True),
        #         ('"malformed) parenthesis (((() string"', True),
        #         ('"quoted_string"', True),
        #         (r'"escape_\character"', False),
        #         (r'"double_\\escape"', True),
        #         (r'"\n"', False),
        #         (r'"\\n"', True),
        #         (r'"unescaped " quote"', False),
        #         (r'"escaped \" quote"', True),
        #     ],
        # ),
        # # String with maximum length
        # (
        #     {"title": "Foo", "type": "string", "maxLength": 3},
        #     f'"{STRING_INNER}{{,3}}"',
        #     [('"ab"', True), ('"a""', False), ('"abcd"', False)],
        # ),
        # # String with minimum length
        # (
        #     {"title": "Foo", "type": "string", "minLength": 3},
        #     f'"{STRING_INNER}{{3,}}"',
        #     [('"ab"', False), ('"abcd"', True), ('"abc""', False)],
        # ),
        # # String with both minimum and maximum length
        # (
        #     {"title": "Foo", "type": "string", "minLength": 3, "maxLength": 5},
        #     f'"{STRING_INNER}{{3,5}}"',
        #     [('"ab"', False), ('"abcd"', True), ('"abcdef""', False)],
        # ),
        # String defined by a regular expression
        (
            {"title": "Foo", "type": "string", "pattern": r"^[a-z]$"},
            r'("[a-z]")',
            [('"a"', True), ('"1"', False)],
        ),
        # Boolean
        (
            {"title": "Foo", "type": "boolean"},
            BOOLEAN,
            [
                ("true", True),
                ("false", True),
                ("null", False),
                ("0", False),
            ],
        ),
        # Null
        (
            {"title": "Foo", "type": "null"},
            NULL,
            [
                ("null", True),
                ("true", False),
                ("0", False),
            ],
        ),
        # Const string
        (
            {"title": "Foo", "const": "Marc", "type": "string"},
            '"Marc"',
            [('"Marc"', True), ('"Jean"', False), ('"John"', False)],
        ),
        # Make sure strings are escaped with regex escaping
        (
            {"title": "Foo", "const": ".*", "type": "string"},
            r'"\.\*"',
            [('".*"', True), (r'"\s*"', False), (r'"\.\*"', False)],
        ),
        # Make sure strings are escaped with JSON escaping
        (
            {"title": "Foo", "const": '"', "type": "string"},
            r'"\\""',
            [('"\\""', True), ('"""', False)],
        ),
        # Const integer
        (
            {"title": "Foo", "const": 0, "type": "integer"},
            "0",
            [("0", True), ("1", False), ("a", False)],
        ),
        # Const float
        (
            {"title": "Foo", "const": 0.2, "type": "float"},
            r"0\.2",
            [("0.2", True), ("032", False)],
        ),
        # Const boolean
        (
            {"title": "Foo", "const": True, "type": "boolean"},
            "true",
            [("true", True), ("True", False)],
        ),
        # Const null
        (
            {"title": "Foo", "const": None, "type": "null"},
            "null",
            [("null", True), ("None", False), ("", False)],
        ),
        # TODO: very close - just nested
        # Enum string
        (
            {"title": "Foo", "enum": ["Marc", "Jean"], "type": "string"},
            # '("Marc"|"Jean")',
            "(A|B)",
            [('"Marc"', True), ('"Jean"', True), ('"John"', False)],
        ),
        # TODO: very close - just nested
        # Make sure strings are escaped with regex and JSON escaping
        (
            {"title": "Foo", "enum": [".*", r"\s*"], "type": "string"},
            r'("\.\*"|"\\\\s\*")',
            [('".*"', True), (r'"\\s*"', True), (r'"\.\*"', False)],
        ),
        # TODO: very close - just nested
        # Enum integer
        (
            {"title": "Foo", "enum": [0, 1], "type": "integer"},
            "(0|1)",
            [("0", True), ("1", True), ("a", False)],
        ),
        # Enum mix of types
        (
            {"title": "Foo", "enum": [6, 5.3, "potato", True, None]},
            r'(6|5\.3|"potato"|true|null)',
            [
                ("6", True),
                ("5.3", True),
                ('"potato"', True),
                ("true", True),
                ("null", True),
                ("523", False),
                ("True", False),
                ("None", False),
            ],
        ),
        # integer
        (
            {
                "title": "Foo",
                "type": "object",
                "properties": {"count": {"title": "Count", "type": "integer"}},
                "required": ["count"],
            },
            '\\{[ ]?"count"[ ]?:[ ]?(-)?(0|[1-9][0-9]*)[ ]?\\}',
            [('{ "count": 100 }', True)],
        ),
        # integer with minimum digits
        (
            {
                "title": "Foo",
                "type": "object",
                "properties": {
                    "count": {"title": "Count", "type": "integer", "minDigits": 3}
                },
                "required": ["count"],
            },
            '\\{[ ]?"count"[ ]?:[ ]?(-)?(0|[1-9][0-9]{2,})[ ]?\\}',
            [('{ "count": 10 }', False), ('{ "count": 100 }', True)],
        ),
        # integer with maximum digits
        (
            {
                "title": "Foo",
                "type": "object",
                "properties": {
                    "count": {"title": "Count", "type": "integer", "maxDigits": 3}
                },
                "required": ["count"],
            },
            '\\{[ ]?"count"[ ]?:[ ]?(-)?(0|[1-9][0-9]{,2})[ ]?\\}',
            [('{ "count": 100 }', True), ('{ "count": 1000 }', False)],
        ),
        # integer with minimum and maximum digits
        (
            {
                "title": "Foo",
                "type": "object",
                "properties": {
                    "count": {
                        "title": "Count",
                        "type": "integer",
                        "minDigits": 3,
                        "maxDigits": 5,
                    }
                },
                "required": ["count"],
            },
            '\\{[ ]?"count"[ ]?:[ ]?(-)?(0|[1-9][0-9]{2,4})[ ]?\\}',
            [
                ('{ "count": 10 }', False),
                ('{ "count": 100 }', True),
                ('{ "count": 10000 }', True),
                ('{ "count": 100000 }', False),
            ],
        ),
        # number
        (
            {
                "title": "Foo",
                "type": "object",
                "properties": {"count": {"title": "Count", "type": "number"}},
                "required": ["count"],
            },
            '\\{[ ]?"count"[ ]?:[ ]?((-)?(0|[1-9][0-9]*))(\\.[0-9]+)?([eE][+-][0-9]+)?[ ]?\\}',
            [('{ "count": 100 }', True), ('{ "count": 100.5 }', True)],
        ),
        # number with min and max integer digits
        (
            {
                "title": "Foo",
                "type": "object",
                "properties": {
                    "count": {
                        "title": "Count",
                        "type": "number",
                        "minDigitsInteger": 3,
                        "maxDigitsInteger": 5,
                    }
                },
                "required": ["count"],
            },
            '\\{[ ]?"count"[ ]?:[ ]?((-)?(0|[1-9][0-9]{2,4}))(\\.[0-9]+)?([eE][+-][0-9]+)?[ ]?\\}',
            [
                ('{ "count": 10.005 }', False),
                ('{ "count": 100.005 }', True),
                ('{ "count": 10000.005 }', True),
                ('{ "count": 100000.005 }', False),
            ],
        ),
        # number with min and max fraction digits
        (
            {
                "title": "Foo",
                "type": "object",
                "properties": {
                    "count": {
                        "title": "Count",
                        "type": "number",
                        "minDigitsFraction": 3,
                        "maxDigitsFraction": 5,
                    }
                },
                "required": ["count"],
            },
            '\\{[ ]?"count"[ ]?:[ ]?((-)?(0|[1-9][0-9]*))(\\.[0-9]{3,5})?([eE][+-][0-9]+)?[ ]?\\}',
            [
                ('{ "count": 1.05 }', False),
                ('{ "count": 1.005 }', True),
                ('{ "count": 1.00005 }', True),
                ('{ "count": 1.000005 }', False),
            ],
        ),
        # number with min and max exponent digits
        (
            {
                "title": "Foo",
                "type": "object",
                "properties": {
                    "count": {
                        "title": "Count",
                        "type": "number",
                        "minDigitsExponent": 3,
                        "maxDigitsExponent": 5,
                    }
                },
                "required": ["count"],
            },
            '\\{[ ]?"count"[ ]?:[ ]?((-)?(0|[1-9][0-9]*))(\\.[0-9]+)?([eE][+-][0-9]{3,5})?[ ]?\\}',
            [
                ('{ "count": 1.05e1 }', False),
                ('{ "count": 1.05e+001 }', True),
                ('{ "count": 1.05e-00001 }', True),
                ('{ "count": 1.05e0000001 }', False),
            ],
        ),
        # number with min and max integer, fraction and exponent digits
        (
            {
                "title": "Foo",
                "type": "object",
                "properties": {
                    "count": {
                        "title": "Count",
                        "type": "number",
                        "minDigitsInteger": 3,
                        "maxDigitsInteger": 5,
                        "minDigitsFraction": 3,
                        "maxDigitsFraction": 5,
                        "minDigitsExponent": 3,
                        "maxDigitsExponent": 5,
                    }
                },
                "required": ["count"],
            },
            '\\{[ ]?"count"[ ]?:[ ]?((-)?(0|[1-9][0-9]{2,4}))(\\.[0-9]{3,5})?([eE][+-][0-9]{3,5})?[ ]?\\}',
            [
                ('{ "count": 1.05e1 }', False),
                ('{ "count": 100.005e+001 }', True),
                ('{ "count": 10000.00005e-00001 }', True),
                ('{ "count": 100000.000005e0000001 }', False),
            ],
        ),
        # array
        (
            {"title": "Foo", "type": "array", "items": {"type": "number"}},
            rf"\[{WHITESPACE}(({NUMBER})(,{WHITESPACE}({NUMBER})){{0,}})?{WHITESPACE}\]",
            [("[1e+9,1.3]", True), ("[]", True), ("[1", False)],
        ),
        # array with a set length of 1
        (
            {
                "title": "Foo",
                "type": "array",
                "items": {"type": "integer"},
                "minItems": 1,
                "maxItems": 1,
            },
            rf"\[{WHITESPACE}(({INTEGER})(,{WHITESPACE}({INTEGER})){{0,0}}){WHITESPACE}\]",
            [("[1]", True), ("[1,2]", False), ('["a"]', False), ("[]", False)],
        ),
        # array with a set length greather than 1
        (
            {
                "title": "Foo",
                "type": "array",
                "items": {"type": "integer"},
                "minItems": 3,
                "maxItems": 3,
            },
            rf"\[{WHITESPACE}(({INTEGER})(,{WHITESPACE}({INTEGER})){{2,2}}){WHITESPACE}\]",
            [("[1]", False), ("[]", False), ("[1,2,3]", True), ("[1,2,3,4]", False)],
        ),
        # array with length 0
        (
            {
                "title": "Foo",
                "type": "array",
                "items": {"type": "integer"},
                "minItems": 0,
                "maxItems": 0,
            },
            rf"\[{WHITESPACE}\]",
            [("[1]", False), ("[]", True), ("[1,2,3]", False), ("[1,2,3,4]", False)],
        ),
        # # object
        # (
        #     {
        #         "title": "TestSchema",
        #         "type": "object",
        #         "properties": {
        #             "test_dict": {
        #                 "title": "Test Dict",
        #                 "additionalProperties": {"type": "string"},
        #                 "type": "object",
        #             }
        #         },
        #         "required": ["test_dict"],
        #     },
        #     rf"""\{{{WHITESPACE}"test_dict"{WHITESPACE}:{WHITESPACE}\{{{WHITESPACE}({STRING}{WHITESPACE}:{WHITESPACE}{STRING}({WHITESPACE},{WHITESPACE}{STRING}{WHITESPACE}:{WHITESPACE}{STRING}){{0,}})?{WHITESPACE}\}}{WHITESPACE}\}}""",
        #     [
        #         ("""{ "test_dict":{"foo":"bar","baz": "bif"}}""", True),
        #         ("""{ "test_dict":{"foo":"bar" }}""", True),
        #         ("""{ "test_dict":{}}""", True),
        #         ("""{ "WRONG_KEY":{}}""", False),
        #         ("""{ "test_dict":{"wrong_type" 1}}""", False),
        #     ],
        # ),
        # # object containing object
        # (
        #     {
        #         "title": "TestSchema",
        #         "type": "object",
        #         "properties": {
        #             "test_dict": {
        #                 "title": "Test Dict",
        #                 "additionalProperties": {
        #                     "additionalProperties": {"type": "integer"},
        #                     "type": "object",
        #                 },
        #                 "type": "object",
        #             }
        #         },
        #         "required": ["test_dict"],
        #     },
        #     rf"""\{{{WHITESPACE}"test_dict"{WHITESPACE}:{WHITESPACE}\{{{WHITESPACE}({STRING}{WHITESPACE}:{WHITESPACE}\{{{WHITESPACE}({STRING}{WHITESPACE}:{WHITESPACE}{INTEGER}({WHITESPACE},{WHITESPACE}{STRING}{WHITESPACE}:{WHITESPACE}{INTEGER}){{0,}})?{WHITESPACE}\}}({WHITESPACE},{WHITESPACE}{STRING}{WHITESPACE}:{WHITESPACE}\{{{WHITESPACE}({STRING}{WHITESPACE}:{WHITESPACE}{INTEGER}({WHITESPACE},{WHITESPACE}{STRING}{WHITESPACE}:{WHITESPACE}{INTEGER}){{0,}})?{WHITESPACE}\}}){{0,}})?{WHITESPACE}\}}{WHITESPACE}\}}""",
        #     [
        #         (
        #             """{"test_dict": {"foo": {"bar": 123, "apple": 99}, "baz": {"bif": 456}}}""",
        #             True,
        #         ),
        #         (
        #             """{"test_dict": {"anykey": {"anykey": 123}, "anykey2": {"bif": 456}}}""",
        #             True,
        #         ),
        #         ("""{"test_dict": {}}""", True),
        #         ("""{"test_dict": {"dict of empty dicts are ok": {} }}""", True),
        #         (
        #             """{"test_dict": {"anykey": {"ONLY Dict[Dict]": 123}, "No Dict[int]" 1: }}""",
        #             False,
        #         ),
        #     ],
        # ),
        # # oneOf
        # (
        #     {
        #         "title": "Foo",
        #         "oneOf": [{"type": "string"}, {"type": "number"}, {"type": "boolean"}],
        #     },
        #     rf'((?:"{STRING_INNER}*")|(?:{NUMBER})|(?:{BOOLEAN}))',
        #     [
        #         ("12.3", True),
        #         ("true", True),
        #         ('"a"', True),
        #         ("null", False),
        #         ("", False),
        #         ("12true", False),
        #         ('1.3"a"', False),
        #         ('12.3true"a"', False),
        #     ],
        # ),
        # # anyOf
        # (
        #     {
        #         "title": "Foo",
        #         "anyOf": [{"type": "string"}, {"type": "integer"}],
        #     },
        #     rf"({STRING}|{INTEGER})",
        #     [("12", True), ('"a"', True), ('1"a"', False)],
        # ),
        # # allOf
        # (
        #     {
        #         "title": "Foo",
        #         "allOf": [{"type": "string"}, {"type": "integer"}],
        #     },
        #     rf"({STRING}{INTEGER})",
        #     [('"a"1', True), ('"a"', False), ('"1"', False)],
        # ),
        # # Tuple / prefixItems
        # (
        #     {
        #         "title": "Foo",
        #         "prefixItems": [{"type": "string"}, {"type": "integer"}],
        #     },
        #     rf"\[{WHITESPACE}{STRING}{WHITESPACE},{WHITESPACE}{INTEGER}{WHITESPACE}\]",
        #     [('["a", 1]', True), ('["a", 1, 1]', False), ("[]", False)],
        # ),
        # Nested schema
        (
            {
                "title": "Bar",
                "type": "object",
                "properties": {
                    "fuzz": {
                        "title": "Foo",
                        "type": "object",
                        "properties": {"spam": {"title": "Spam", "type": "integer"}},
                        "required": ["spam"],
                    }
                },
                "required": ["fuzz"],
            },
            f'\\{{[ ]?"fuzz"[ ]?:[ ]?\\{{[ ]?"spam"[ ]?:[ ]?{INTEGER}[ ]?\\}}[ ]?\\}}',
            [('{ "fuzz": { "spam": 100 }}', True)],
        ),
        # # Schema with a reference
        # (
        #     {
        #         "title": "User",
        #         "type": "object",
        #         "properties": {
        #             "user_id": {"title": "User Id", "type": "integer"},
        #             "name": {"title": "Name", "type": "string"},
        #             "a": {"$ref": "#/properties/name"},
        #         },
        #         "required": ["user_id", "name", "a"],
        #     },
        #     f'\\{{[ ]?"user_id"[ ]?:[ ]?{INTEGER}[ ]?,[ ]?"name"[ ]?:[ ]?{STRING}[ ]?,[ ]?"a"[ ]?:[ ]?{STRING}[ ]?\\}}',
        #     [('{"user_id": 100, "name": "John", "a": "Marc"}', True)],
        # ),
        # (
        #     {
        #         "title": "User",
        #         "type": "object",
        #         "$defs": {"name": {"title": "Name2", "type": "string"}},
        #         "properties": {
        #             "user_id": {"title": "User Id", "type": "integer"},
        #             "name": {"title": "Name", "type": "string"},
        #             "name2": {"$ref": "#/$defs/name"},
        #         },
        #         "required": ["user_id", "name", "name2"],
        #     },
        #     f'\\{{[ ]?"user_id"[ ]?:[ ]?{INTEGER}[ ]?,[ ]?"name"[ ]?:[ ]?{STRING}[ ]?,[ ]?"name2"[ ]?:[ ]?{STRING}[ ]?\\}}',
        #     [('{"user_id": 100, "name": "John", "name2": "Marc"}', True)],
        # ),
        # (
        #     {
        #         "$id": "customer",
        #         "$schema": "https://json-schema.org/draft/2020-12/schema",
        #         "title": "Customer",
        #         "type": "object",
        #         "properties": {
        #             "name": {"type": "string"},
        #             "last_name": {"type": "string"},
        #             "address": {"$ref": "customer#/$defs/address"},
        #         },
        #         "required": [
        #             "name",
        #             "first_name",
        #             "last_name",
        #             "address",
        #             "shipping_address",
        #             "billing_address",
        #         ],
        #         "$defs": {
        #             "address": {
        #                 "title": "Address",
        #                 "$schema": "http://json-schema.org/draft-07/schema#",
        #                 "type": "object",
        #                 "properties": {
        #                     "city": {"type": "string"},
        #                 },
        #                 "required": ["street_address", "city", "state"],
        #                 "definitions": {
        #                     "state": {
        #                         "type": "object",
        #                         "title": "State",
        #                         "properties": {"name": {"type": "string"}},
        #                         "required": ["name"],
        #                     }
        #                 },
        #             }
        #         },
        #     },
        #     f'\\{{[ ]?"name"[ ]?:[ ]?{STRING}[ ]?,[ ]?"last_name"[ ]?:[ ]?{STRING}[ ]?,[ ]?"address"[ ]?:[ ]?\\{{[ ]?"city"[ ]?:[ ]?{STRING}[ ]?\\}}[ ]?\\}}',
        #     [
        #         (
        #             '{"name": "John", "last_name": "Doe", "address": {"city": "Paris"}}',
        #             True,
        #         )
        #     ],
        # ),
        # # Optional properties
        # # Last required property in first position
        # (
        #     {
        #         "properties": {
        #             "name": {"type": "string"},
        #             "age": {"anyOf": [{"type": "integer"}, {"type": "null"}]},
        #             "weapon": {"anyOf": [{"type": "string"}, {"type": "null"}]},
        #         },
        #         "required": ["name"],
        #         "title": "Character",
        #         "type": "object",
        #     },
        #     f'\\{{[ ]?"name"[ ]?:[ ]?{STRING}([ ]?,[ ]?"age"[ ]?:[ ]?({INTEGER}|null))?([ ]?,[ ]?"weapon"[ ]?:[ ]?({STRING}|null))?[ ]?\\}}',
        #     [
        #         ('{ "name" : "Player" }', True),
        #         ('{ "name" : "Player", "weapon" : "sword" }', True),
        #         ('{ "age" : 10, "weapon" : "sword" }', False),
        #     ],
        # ),
        # # Last required property in middle position
        # (
        #     {
        #         "properties": {
        #             "name": {"type": "string"},
        #             "age": {"anyOf": [{"type": "integer"}, {"type": "null"}]},
        #             "weapon": {"type": "string"},
        #             "strength": {"anyOf": [{"type": "integer"}, {"type": "null"}]},
        #         },
        #         "required": ["name", "weapon"],
        #         "title": "Character",
        #         "type": "object",
        #     },
        #     f'\\{{[ ]?"name"[ ]?:[ ]?{STRING}[ ]?,([ ]?"age"[ ]?:[ ]?({INTEGER}|null)[ ]?,)?[ ]?"weapon"[ ]?:[ ]?{STRING}([ ]?,[ ]?"strength"[ ]?:[ ]?({INTEGER}|null))?[ ]?\\}}',
        #     [
        #         ('{ "name" : "Player" , "weapon" : "sword" }', True),
        #         (
        #             '{ "name" : "Player", "age" : 10, "weapon" : "sword" , "strength" : 10 }',
        #             True,
        #         ),
        #         ('{ "weapon" : "sword" }', False),
        #     ],
        # ),
        # # Last required property in last position
        # (
        #     {
        #         "properties": {
        #             "name": {"anyOf": [{"type": "string"}, {"type": "null"}]},
        #             "age": {"type": "integer"},
        #             "armor": {"type": "string"},
        #             "strength": {"anyOf": [{"type": "integer"}, {"type": "null"}]},
        #             "weapon": {"title": "Weapon", "type": "string"},
        #         },
        #         "required": ["age", "armor", "weapon"],
        #         "title": "Character",
        #         "type": "object",
        #     },
        #     f'\\{{([ ]?"name"[ ]?:[ ]?({STRING}|null)[ ]?,)?[ ]?"age"[ ]?:[ ]?{INTEGER}[ ]?,[ ]?"armor"[ ]?:[ ]?{STRING}[ ]?,([ ]?"strength"[ ]?:[ ]?({INTEGER}|null)[ ]?,)?[ ]?"weapon"[ ]?:[ ]?{STRING}[ ]?\\}}',
        #     [
        #         (
        #             '{ "name" : "Player", "age" : 10, "armor" : "plate", "strength" : 11, "weapon" : "sword" }',
        #             True,
        #         ),
        #         ('{ "age" : 10, "armor" : "plate", "weapon" : "sword" }', True),
        #         (
        #             '{ "name" : "Kahlhanbeh", "armor" : "plate", "weapon" : "sword" }',
        #             False,
        #         ),
        #     ],
        # ),
        # # All properties are optional
        # (
        #     {
        #         "properties": {
        #             "name": {"anyOf": [{"type": "string"}, {"type": "null"}]},
        #             "age": {"anyOf": [{"type": "integer"}, {"type": "null"}]},
        #             "strength": {"anyOf": [{"type": "integer"}, {"type": "null"}]},
        #         },
        #         "title": "Character",
        #         "type": "object",
        #     },
        #     f'\\{{([ ]?"name"[ ]?:[ ]?({STRING}|null)([ ]?,[ ]?"age"[ ]?:[ ]?({INTEGER}|null))?([ ]?,[ ]?"strength"[ ]?:[ ]?({INTEGER}|null))?|([ ]?"name"[ ]?:[ ]?({STRING}|null)[ ]?,)?[ ]?"age"[ ]?:[ ]?({INTEGER}|null)([ ]?,[ ]?"strength"[ ]?:[ ]?({INTEGER}|null))?|([ ]?"name"[ ]?:[ ]?({STRING}|null)[ ]?,)?([ ]?"age"[ ]?:[ ]?({INTEGER}|null)[ ]?,)?[ ]?"strength"[ ]?:[ ]?({INTEGER}|null))?[ ]?\\}}',
        #     [
        #         ('{ "name" : "Player" }', True),
        #         ('{ "name" : "Player", "age" : 10, "strength" : 10 }', True),
        #         ('{ "age" : 10, "strength" : 10 }', True),
        #         ("{ }", True),
        #     ],
        # ),
    ],
)
def test_match(schema, regex, examples):
    pattern = interegular.parse_pattern(regex)
    _pattern = parse_pattern(regex)
    converted = convert_to_interegular(_pattern)

    print("Regex: ", regex)
    print(f"Pattern: \n{pattern}")
    print(f"Converted: \n{converted}")

    assert deep_compare(pattern, converted)
