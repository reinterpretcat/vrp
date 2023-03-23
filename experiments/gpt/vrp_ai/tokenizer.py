from tokenizers import Tokenizer
from tokenizers.models import BPE, Unigram, WordLevel, WordPiece
from tokenizers.trainers import BpeTrainer, WordLevelTrainer, WordPieceTrainer, UnigramTrainer
from tokenizers.pre_tokenizers import Whitespace
from pathlib import Path


tokenizer_ids = ['WLV', 'BPE', 'UNI', 'WPC']
unknown_word_token = "<UNK>"
special_tokens = ["<UNK>", "<SEP>", "<MASK>", "<CLS>"]


def prepare_tokenizer_trainer(alg):
    """
    Prepares the tokenizer and trainer with unknown and special tokens.
    """
    if alg == 'BPE':
        tokenizer = Tokenizer(BPE(unk_token = unknown_word_token))
        trainer = BpeTrainer(special_tokens = special_tokens)
    elif alg == 'UNI':
        tokenizer = Tokenizer(Unigram())
        trainer = UnigramTrainer(unk_token = unknown_word_token, special_tokens = special_tokens)
    elif alg == 'WPC':
        tokenizer = Tokenizer(WordPiece(unk_token = unknown_word_token))
        trainer = WordPieceTrainer(special_tokens = special_tokens)
    else:
        tokenizer = Tokenizer(WordLevel(unk_token = unknown_word_token))
        trainer = WordLevelTrainer(special_tokens = special_tokens)
    
    tokenizer.pre_tokenizer = Whitespace()

    return tokenizer, trainer


def train_tokenizer(files, cache_dir, alg ='WLV'):
    """
    Takes the files and trains the tokenizer on them.
    """
    tokenizer, trainer = prepare_tokenizer_trainer(alg)
    tokenizer.train(files, trainer)
    tokenizer.save(str(Path(cache_dir).joinpath(f"tokenizer-trained_{alg}.json")))
    
    tokenizer = load_tokenizer_from_dir(cache_dir, alg)

    return tokenizer


def load_tokenizer_from_dir(cache_dir, alg):
    """
    Loads tokenizer from directory
    """
    file_path = str(Path(cache_dir).joinpath(f"tokenizer-trained_{alg}.json"))
    tokenizer = Tokenizer.from_file(file_path)
    
    return tokenizer


def tokenize(input_string, tokenizer):
    """
    Tokenizes the input string using the tokenizer.
    """
    output = tokenizer.encode(input_string)

    return output