import json
import unittest

from pydantic import ValidationError

from vrp_cli.models.config import Config
from vrp_cli.models.solution import Timing


class ModelContractTest(unittest.TestCase):
    def test_serializes_python_safe_name_with_json_alias(self) -> None:
        timing = Timing.model_validate(
            {
                "break": 1,
                "commuting": 2,
                "driving": 3,
                "parking": 4,
                "serving": 5,
                "waiting": 6,
            }
        )

        dumped = json.loads(timing.model_dump_json())

        self.assertEqual(dumped["break"], 1)
        self.assertNotIn("break_", dumped)

    def test_rejects_unknown_fields(self) -> None:
        with self.assertRaises(ValidationError):
            Config.model_validate({"unknownSetting": True})


if __name__ == "__main__":
    unittest.main()
