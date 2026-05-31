import { Router } from 'express';
import { login, register } from '../controllers/auth.controller.js';
import { walletChallenge, walletVerify } from '../controllers/wallet.controller.js';
import {
  loginSchema,
  registerSchema,
  validateBody,
  walletChallengeSchema,
  walletVerifySchema,
} from '../validators/auth.validator.js';

const router = Router();

router.post('/register', validateBody(registerSchema), register);
router.post('/login', validateBody(loginSchema), login);
router.post('/wallet/challenge', validateBody(walletChallengeSchema), walletChallenge);
router.post('/wallet/verify', validateBody(walletVerifySchema), walletVerify);

export default router;
