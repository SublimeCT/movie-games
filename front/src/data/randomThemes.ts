export interface RandomThemeConfig {
  theme: string;
  genre?: string[];
  synopsis?: string;
  characters?: Array<{
    name: string;
    gender: string;
    description: string;
    isMain: boolean;
  }>;
}

export const randomThemes: RandomThemeConfig[] = [
  {
    theme: '升职前一晚',
    genre: ['职场', '剧情', '悬疑'],
    synopsis:
      '深夜十一点，办公室只剩下我还在加班。明天就是期待已久的部门经理面试。整理文件时，一份标注"机密"的文件夹掉了出来——里面竟然有王总挪用公款的证据。这时，走廊里传来了脚步声，我必须立刻做出选择：举报还是沉默？这份证据或许是我升职的筹码，但也可能让我万劫不复。',
    characters: [
      {
        name: '陈默',
        gender: '男',
        description:
          '28岁，普通职员。渴望改变现状，正直但面临诱惑，手握王总挪用公款的证据。',
        isMain: true,
      },
      {
        name: '王总',
        gender: '男',
        description:
          '45岁，公司总经理。表面和蔼，实则手段狠辣，挪用公款的秘密一旦暴露将身败名裂。',
        isMain: false,
      },
      {
        name: '李雅琪',
        gender: '女',
        description:
          '27岁，竞争对手。聪明善妒，为达目的不择手段，似乎也察觉到了什么。',
        isMain: false,
      },
      {
        name: '老张',
        gender: '男',
        description:
          '50岁，老员工。知道公司内幕但明哲保身，关键时刻可能成为盟友或障碍。',
        isMain: false,
      },
    ],
  },
  {
    theme: '重生之被班主任拍醒',
    genre: ['青春', '喜剧', '剧情'],
    synopsis:
      '数学课，我趴在课桌上流口水。突然，后脑勺挨了一巴掌："又睡觉！"猛一抬头，我惊恐地发现——黑板上写着"2014年"，眼前是年轻十岁的班主任老张。我竟然重生了！带着30岁社畜的记忆回到高三，我发誓要改变高考失利的命运，重新追回错过的暗恋，但事情真的会如我所愿吗？',
    characters: [
      {
        name: '林小北',
        gender: '男',
        description:
          '18岁，高三学生（内心30岁）。带着未来的记忆重生，决心改变命运，但成人思维与少年身份格格不入。',
        isMain: true,
      },
      {
        name: '张老师',
        gender: '男',
        description:
          '40岁，数学老师兼班主任。严厉古板，口头禅"你们是我带过最差的一届"，其实关心学生。',
        isMain: false,
      },
      {
        name: '苏小美',
        gender: '女',
        description:
          '18岁，同桌。当年的暗恋对象，活泼可爱，数学很差但很努力，不知道自己已被"预支"了未来。',
        isMain: false,
      },
      {
        name: '胖子大伟',
        gender: '男',
        description:
          '18岁，死党。讲义气但成绩烂，未来会出国发展，现在是需要帮助的好兄弟。',
        isMain: false,
      },
    ],
  },
  {
    theme: '电梯停在中间',
    genre: ['悬疑', '惊悚', '剧情'],
    synopsis:
      '加班到凌晨两点，我独自走进公司电梯。按下一楼的按钮，电梯却缓缓上行...停在了一个不存在的楼层。电梯门缓缓打开，外面是一片漆黑，只有远处传来诡异的哭声。手机没有信号，电梯门关不上，我被迫走出电梯，探索这个隐藏在写字楼中的诡异空间。这里到底隐藏着什么秘密？',
    characters: [
      {
        name: '周远',
        gender: '男',
        description:
          '26岁，程序员。深夜加班被困诡异楼层，手机无信号，必须找到出路逃离。',
        isMain: true,
      },
      {
        name: '保安老王',
        gender: '男',
        description:
          '55岁，大楼保安。似乎知道这个楼层的秘密，但说话吞吞吐吐，神色慌张。',
        isMain: false,
      },
      {
        name: '红衣女孩',
        gender: '女',
        description:
          '??岁，神秘存在。总是出现在视野边缘，似乎在指引方向，又似乎在诱导陷阱。',
        isMain: false,
      },
    ],
  },
  {
    theme: '末日避难所的最后一张门票',
    genre: ['科幻', '冒险', '伦理'],
    synopsis:
      '小行星撞击倒计时24小时。政府秘密建立的地下避难所即将关闭，而我手里只有最后一张多余的门票。我的面前是：身患绝症但深爱我的妻子、天赋异禀可能拯救人类的年轻科学家、以及我那只有5岁的女儿。在这最后的时刻，人性的光辉与丑恶同时上演，我该如何选择？',
    characters: [
      {
        name: '李明',
        gender: '男',
        description:
          '35岁，前特种兵。掌握着最后一张避难所门票，面临最艰难的道德抉择。',
        isMain: true,
      },
      {
        name: '张婉',
        gender: '女',
        description:
          '32岁，妻子。身患绝症，温柔体贴，劝丈夫把票给更有需要的人，但眼神中流露着对生的渴望。',
        isMain: false,
      },
      {
        name: '艾伦',
        gender: '男',
        description:
          '24岁，天才物理学家。声称自己有方案能解决撞击后的生态危机，但需要活下去才能实施。',
        isMain: false,
      },
      {
        name: '朵朵',
        gender: '女',
        description:
          '5岁，女儿。天真烂漫，不知道世界即将毁灭，只想着爸爸妈妈永远在一起。',
        isMain: false,
      },
    ],
  },
  {
    theme: '修仙界的卧底魔尊',
    genre: ['武侠', '仙侠', '喜剧'],
    synopsis:
      '我是魔界至尊，为了寻找传说中的神器潜入正道第一大派。为了不暴露身份，我必须表现得比正道还正道。结果用力过猛，我不仅成了掌门的关门弟子，还被推举为正道盟主候选人！眼看魔界大军即将压境，一边是忠心耿耿的魔界部下，一边是视我为希望的正道同门，这戏还要怎么演下去？',
    characters: [
      {
        name: '夜无痕',
        gender: '男',
        description:
          '1000岁（外表20岁），魔尊。为了神器卧底正道，天赋异禀，内心吐槽役，表面高冷男神。',
        isMain: true,
      },
      {
        name: '柳清霜',
        gender: '女',
        description:
          '19岁，正道圣女。清冷出尘，对"夜师弟"暗生情愫，却不知对方是宿敌魔尊。',
        isMain: false,
      },
      {
        name: '赤炎',
        gender: '男',
        description:
          '800岁，魔界左护法。憨厚忠诚，经常偷偷下山给魔尊送情报，却总是搞出乌龙助攻。',
        isMain: false,
      },
    ],
  },
  {
    theme: '我在古代当御厨',
    genre: ['历史', '美食', '爽文'],
    synopsis:
      '米其林三星主厨穿越到了古代御膳房，成了一名打杂的小太监（假的！）。面对刁钻的皇帝、勾心斗角的后宫嫔妃，我凭着一手分子料理和现代烹饪技巧，在宫廷中混得风生水起。但伴君如伴虎，一道"开水白菜"竟牵扯出前朝宝藏的秘密，我手里的菜刀，不仅要切菜，还得保命！',
    characters: [
      {
        name: '小林子',
        gender: '男',
        description:
          '25岁，穿越主厨。机智圆滑，厨艺高超，一心想攒钱出宫开酒楼，却卷入宫廷斗争。',
        isMain: true,
      },
      {
        name: '皇帝',
        gender: '男',
        description:
          '30岁，年轻帝王。嘴刁且腹黑，对这个总能做出新奇美食的小太监充满兴趣和怀疑。',
        isMain: false,
      },
      {
        name: '萧贵妃',
        gender: '女',
        description:
          '22岁，宠妃。娇纵跋扈，是个吃货，为了吃一口新品甜点可以答应任何条件。',
        isMain: false,
      },
    ],
  },
  {
    theme: '赛博朋克：记忆贩卖师',
    genre: ['科幻', '悬疑', '冒险'],
    synopsis:
      '2077年，霓虹闪烁的夜之城。我是一名地下记忆贩卖师，专门提取、修改、贩卖记忆。一天，一位神秘客户重金要求我删除一段记忆，而在操作过程中，我发现这段记忆竟然包含了公司掌控城市的惊天阴谋，更可怕的是，记忆的主角……似乎就是我自己。',
    characters: [
      {
        name: 'K',
        gender: '男',
        description:
          '30岁，记忆贩卖师。冷酷专业，技术顶尖，却丢失了自己的部分过去，一直在寻找真相。',
        isMain: true,
      },
      {
        name: '露娜',
        gender: '女',
        description:
          '25岁，黑客。活泼叛逆，在这个赛博世界中寻找自由，是K唯一信任的搭档。',
        isMain: false,
      },
      {
        name: '代理人',
        gender: '男',
        description:
          '??岁，公司高层。西装革履，面带微笑的杀手，代表着不可撼动的秩序与权力。',
        isMain: false,
      },
    ],
  },
  {
    theme: '荒岛求生：谁是卧底',
    genre: ['冒险', '悬疑', '伦理'],
    synopsis:
      '豪华游轮沉没，我和其他6名幸存者流落荒岛。原本以为是普通的求生，直到第二天早上，我们发现无线电被破坏，淡水被投毒。幸存者中混入了一个想要杀死所有人的疯子！在饥饿、恐慌和猜忌中，我们不仅要对抗大自然，还要找出那个隐藏在身边的恶魔。',
    characters: [
      {
        name: '杰克',
        gender: '男',
        description:
          '29岁，医生。冷静理智，团队的临时领袖，利用医学知识帮助大家生存，但也因此被怀疑。',
        isMain: true,
      },
      {
        name: '安娜',
        gender: '女',
        description:
          '24岁，富家女。起初娇气任性，但在生存压力下逐渐展现出坚韧的一面，直觉敏锐。',
        isMain: false,
      },
      {
        name: '大胡子',
        gender: '男',
        description:
          '45岁，船员。经验丰富但脾气暴躁，掌握着生存技能，却隐瞒了船难的真相。',
        isMain: false,
      },
    ],
  },
];